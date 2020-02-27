// -- terminal.rs --

use {
    crate::{
        servant::{Context, NotifyServant, Oid, Record, ServantResult, ConnectionId},
        sync::{Arc, Condvar, Mutex},
        utilities::DropGuard,
    },
    async_std::{
        net::TcpStream,
        prelude::*,
        stream,
        task,
    },
    codec::RecordCodec,
    futures::{
        channel::mpsc::{unbounded, UnboundedSender},
        pin_mut, select,
        sink::SinkExt,
        FutureExt as _,
    },
    futures_codec::{FramedRead, FramedWrite},
    log::{info, warn},
    std::{
        collections::HashMap,
        time::{Duration, SystemTime},
    },
};

// --

type RecordId = usize;
type Tx = UnboundedSender<Record>;
#[derive(Debug)]
struct _Token {
    m: Mutex<Option<ServantResult<Vec<u8>>>>,
    cv: Condvar,
}
type Token = Arc<_Token>;
type TokenMap = HashMap<RecordId, Token>;
type TokenPool = Vec<Token>;
type NotifyServantEntry = Box<dyn NotifyServant + Send>;

struct CallbackRecord {
    start: SystemTime,
    timeout_ms: u64,
    oid: Option<Oid>,
    callback: Box<dyn Fn(Option<Oid>, ServantResult<Vec<u8>>) + Send>,
}
type CallbackMap = HashMap<RecordId, CallbackRecord>;

struct _Terminal {
    addr: String,
    conn_id: Option<ConnectionId>,
    req_id: RecordId,
    report_id: RecordId,
    invoke_timeout_ms: u64,
    sender: Option<Tx>,
    token_pool: TokenPool,
    token_map: TokenMap,
    max_count_of_callback: usize,
    callback_map: CallbackMap,
    receiver: Option<NotifyServantEntry>,
}
impl _Terminal {
    fn timeout_value_in_context(&self, ctx: &Option<Context>) -> u64 {
        if let Some(c) = ctx.as_ref() {
            if let Some(t) = c.timeout_millisecond {
                t
            } else {
                self.invoke_timeout_ms
            }
        } else {
            self.invoke_timeout_ms
        }
    }
}

#[derive(Clone)]
pub struct Terminal(Arc<Mutex<_Terminal>>);
impl Terminal {
    pub fn new(
        addr: String,
        invoke_timeout_ms: u64,
        token_count_by_terminal: usize,
        max_count_of_callback: usize,
    ) -> Self {
        let mut t = _Terminal {
            addr,
            conn_id: None,
            req_id: 0,
            report_id: 0,
            invoke_timeout_ms,
            sender: None,
            token_pool: TokenPool::new(),
            token_map: TokenMap::new(),
            max_count_of_callback,
            callback_map: CallbackMap::new(),
            receiver: None,
        };
        for _ in 0..token_count_by_terminal {
            let r = _Token {
                m: Mutex::new(None),
                cv: Condvar::default(),
            };
            t.token_pool.push(Arc::new(r));
        }
        Self(Arc::new(Mutex::new(t)))
    }
    pub async fn clean(&self) {
        let mut g = self.0.lock().await;
        g.sender.take();
    }
    pub async fn set_receiver(&self, receiver: NotifyServantEntry) {
        let mut g = self.0.lock().await;
        g.receiver.replace(receiver);
    }
    async fn set_tx(&self, tx: Option<Tx>) {
        let mut g = self.0.lock().await;
        g.sender = tx;
    }
    async fn set_conn_id(&self, id: ConnectionId) {
        let mut g = self.0.lock().await;
        g.conn_id.replace(id);
    }
    pub async fn conn_id(&self) -> Option<ConnectionId> {
        let g = self.0.lock().await;
        g.conn_id.clone()
    }
    async fn tx_or_reconnect(&self) -> ServantResult<Tx> {
        {
            let g = self.0.lock().await;
            if let Some(tx) = g.sender.as_ref() {
                return Ok(tx.clone());
            }
        }

        if let Err(e) = self.connect_to().await {
            return Err(e.to_string().into());
        }

        let g = self.0.lock().await;
        if let Some(tx) = g.sender.as_ref() {
            Ok(tx.clone())
        } else {
            Err("sender is none.".into())
        }
    }
    pub async fn report(&self, oid: Oid, msg: Vec<u8>) -> ServantResult<()> {
        let record = {
            let mut g = self.0.lock().await;
            g.report_id += 1;
            Record::Report {
                id: g.report_id,
                oid,
                msg,
            }
        };
        let mut tx = self.tx_or_reconnect().await?;
        if let Err(e) = tx.send(record).await {
            Err(e.to_string().into())
        } else {
            Ok(())
        }
    }
    pub async fn invoke_with_callback<F>(
        &self,
        ctx: Option<Context>,
        oid: Option<Oid>,
        req: Vec<u8>,
        f: F,
    ) -> ServantResult<()>
    where
        F: 'static + Fn(Option<Oid>, ServantResult<Vec<u8>>) + Send,
    {
        let id = {
            let mut g = self.0.lock().await;
            if g.callback_map.len() >= g.max_count_of_callback {
                return Err("callback map is full.".into());
            }
            g.req_id += 1;
            let id = g.req_id;
            let timeout_ms = g.timeout_value_in_context(&ctx);
            g.callback_map.insert(
                id,
                CallbackRecord {
                    start: SystemTime::now(),
                    oid: oid.clone(),
                    timeout_ms,
                    callback: Box::new(f),
                },
            );
            id
        };
        let record = Record::Request { id, ctx, oid, req };
        let mut tx = self.tx_or_reconnect().await?;
        if let Err(e) = tx.send(record).await {
            let mut g = self.0.lock().await;
            g.callback_map.remove(&id).unwrap();
            Err(e.to_string().into())
        } else {
            Ok(())
        }
    }
    pub async fn invoke(
        &self,
        ctx: Option<Context>,
        oid: Option<Oid>,
        req: Vec<u8>,
    ) -> ServantResult<Vec<u8>> {
        let (index, token, timeout_ms) = {
            let mut g = self.0.lock().await;
            if let Some(tok) = g.token_pool.pop() {
                g.req_id += 1;
                let id = g.req_id;
                g.token_map.insert(id, tok.clone());
                (id, tok, g.timeout_value_in_context(&ctx))
            } else {
                return Err("token pool is empty.".into());
            }
        };
        let record = Record::Request {
            id: index,
            ctx,
            oid,
            req,
        };
        let ret = match self.tx_or_reconnect().await {
            Err(e) => Err(e),
            Ok(mut tx) => if let Err(e) = tx.send(record).await {
                Err(e.to_string().into())
            } else {
                let g = token.m.lock().await;
                let mut r = token
                    .cv
                    .wait_timeout(g, Duration::from_millis(timeout_ms))
                    .await;
                if r.1.timed_out() {
                    Err("timed_out.".into())
                } else {
                    r.0.take().unwrap()
                }
            },
        };
        {
            let mut g = self.0.lock().await;
            g.token_map.remove(&index);
            g.token_pool.push(token);
        }
        ret
    }
    async fn received(&self, record: Record) {
        match record {
            Record::Notice { id, msg } => {
                let _id = id;
                let mut g = self.0.lock().await;
                if let Some(receiver) = g.receiver.as_mut() {
                    receiver.serve(msg);
                }
            }
            Record::Response { id, oid, ret } => {
                let _oid = oid;
                let (token, callback) = {
                    let mut g = self.0.lock().await;
                    (g.token_map.remove(&id), g.callback_map.remove(&id))
                };
                let ret = match bincode::deserialize(&ret) {
                    Ok(ret) => ret,
                    Err(e) => Err(e.to_string().into()),
                };
                if let Some(token) = token {
                    let mut g = token.m.lock().await;
                    g.replace(ret);
                    token.cv.notify_one();
                } else if let Some(r) = callback {
                    (r.callback)(r.oid, ret);
                } else {
                    warn!(
                        "received {:?}, but can't find id: {} in token map and callback map.",
                        ret, id
                    );
                }
            }
            Record::Report { .. } => unreachable!(),
            Record::Request { .. } => unreachable!(),
        }
    }
    pub async fn connect_to(&self) -> std::io::Result<()> {
        let stream = {
            let g = self.0.lock().await;
            TcpStream::connect(&g.addr).await?
        };
        info!("connected to {}", stream.peer_addr()?);

        let conn = self.clone();
        task::spawn(async move {
            let r = conn.run(stream).await;
            info!("terminal run result: {:?}", r);
        });
        loop {
            task::sleep(Duration::from_micros(10)).await;
            if self.0.lock().await.sender.is_some() {
                break;
            }
        }
        Ok(())
    }
    async fn run(&self, stream: TcpStream) -> std::io::Result<()> {
        #[derive(Debug)]
        enum SelectedValue {
            ReadNone,
            WriteNone,
            Tick,
            Read(Record),
            Write(Record),
        }

        let (tx, rx) = unbounded();
        self.set_tx(Some(tx)).await;
        self.set_conn_id(stream.local_addr()?).await;
        let _terminal_clean = DropGuard::new(self.clone(), |t| {
            task::block_on(async move {
                info!("terminal quit.");
                t.clean().await;
            });
        });

        let (reader, writer) = (&stream, &stream);
        let read_framed = FramedRead::new(reader, RecordCodec::<u32, Record>::default());
        let mut write_framed = FramedWrite::new(writer, RecordCodec::<u32, Record>::default());
        let interval = stream::interval(Duration::from_millis(1000));

        pin_mut!(read_framed, rx, interval);
        loop {
            let value = select! {
                from_adapter = read_framed.next().fuse() => match from_adapter {
                    Some(record) => SelectedValue::Read(record?),
                    None => SelectedValue::ReadNone,
                },
                to_adapter = rx.next().fuse() => match to_adapter {
                    Some(record) => SelectedValue::Write(record),
                    None => SelectedValue::WriteNone,
                },
                _tick = interval.next().fuse() => SelectedValue::Tick,
            };

            match value {
                SelectedValue::Read(record) => self.received(record).await,
                SelectedValue::Write(record) => write_framed.send(record).await?,
                SelectedValue::Tick => self.tick().await,
                _ => {
                    info!("loop break due to SelectedValue: {:?}", value);
                    break;
                }
            }
        }
        Ok(())
    }
    async fn tick(&self) {
        let now = SystemTime::now();
        let mut g = self.0.lock().await;
        let v: Vec<RecordId> = g
            .callback_map
            .iter()
            .map(|(id, record)| {
                if record.start <= now - Duration::from_millis(record.timeout_ms) {
                    Some(*id)
                } else {
                    None
                }
            })
            .filter(|x| x.is_some())
            .map(|x| x.unwrap())
            .collect();
        v.iter().for_each(|x| {
            if let Some(record) = g.callback_map.remove(&x) {
                (record.callback)(record.oid, Err("timeout in callback.".into()));
            }
        });
    }
    pub fn proxy<T, F>(&self, name: &str, f: F) -> T
    where
        F: Fn(&str, &Terminal) -> T,
    {
        f(name, self)
    }
    pub fn proxy_with_context<T, F>(&self, ctx: Context, name: &str, f: F) -> T
    where
        F: Fn(Context, &str, &Terminal) -> T,
    {
        f(ctx, name, self)
    }
}
