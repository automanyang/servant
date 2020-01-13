// -- terminal.rs --

use {
    super::{
        drop_guard::DropGuard,
        servant::{Context, NotifyServant, Oid, Record, ServantResult},
    },
    async_std::{
        net::{TcpStream, ToSocketAddrs},
        prelude::*,
        stream,
        sync::{Arc, Mutex},
        task::{self, JoinHandle},
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
        sync::{Condvar, Mutex as StdMutex},
        time::{Duration, SystemTime},
    },
};

// --

const DEFAULT_TIMEOUT_MS: u64 = 5000;
fn timeout_value_in_context(ctx: &Option<Context>) -> u64 {
    if let Some(c) = ctx.as_ref() {
        if let Some(t) = c.timeout_millisecond {
            t
        } else {
            DEFAULT_TIMEOUT_MS
        }
    } else {
        DEFAULT_TIMEOUT_MS
    }
}

type RecordId = usize;
type Tx = UnboundedSender<Record>;
#[derive(Debug)]
struct _Token {
    m: StdMutex<Option<ServantResult<Vec<u8>>>>,
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
    req_id: RecordId,
    report_id: RecordId,
    sender: Option<Tx>,
    token_pool: TokenPool,
    token_map: TokenMap,
    callback_map: CallbackMap,
    receiver: Option<NotifyServantEntry>,
}

#[derive(Clone)]
pub struct Terminal(Arc<Mutex<_Terminal>>);
impl Terminal {
    pub fn new(token_pool_size: usize, receiver: Option<NotifyServantEntry>) -> Self {
        let mut t = _Terminal {
            req_id: 0,
            report_id: 0,
            sender: None,
            token_pool: TokenPool::new(),
            token_map: TokenMap::new(),
            callback_map: CallbackMap::new(),
            receiver,
        };
        for _ in 0..token_pool_size {
            let r = _Token {
                m: StdMutex::new(None),
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
    async fn set_tx(&self, tx: Option<Tx>) {
        let mut g = self.0.lock().await;
        g.sender = tx;
    }
    pub async fn report(&self, oid: Oid, msg: Vec<u8>) -> ServantResult<()> {
        let mut g = self.0.lock().await;
        g.report_id += 1;
        if let Some(mut tx) = g.sender.as_ref() {
            let record = Record::Report {
                id: g.report_id,
                oid,
                msg,
            };
            if let Err(e) = tx.send(record).await {
                Err(e.to_string().into())
            } else {
                Ok(())
            }
        } else {
            Err("sender is none.".into())
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
        let mut g = self.0.lock().await;
        let mut tx = if let Some(tx) = g.sender.as_ref() {
            tx.clone()
        } else {
            return Err("sender is none.".into());
        };
        g.req_id += 1;
        let id = g.req_id;
        assert_eq!(
            true,
            g.callback_map
                .insert(
                    id,
                    CallbackRecord {
                        start: SystemTime::now(),
                        oid: oid.clone(),
                        timeout_ms: timeout_value_in_context(&ctx),
                        callback: Box::new(f)
                    }
                )
                .is_none()
        );
        let record = Record::Request { id, ctx, oid, req };
        if let Err(e) = tx.send(record).await {
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
        let (mut tx, index, token) = {
            let mut g = self.0.lock().await;
            let tx = if let Some(tx) = g.sender.as_ref() {
                tx.clone()
            } else {
                return Err("sender is none.".into());
            };
            if let Some(tok) = g.token_pool.pop() {
                g.req_id += 1;
                let id = g.req_id;
                g.token_map.insert(id, tok.clone());
                (tx, id, tok)
            } else {
                return Err("token pool is empty.".into());
            }
        };
        let ret = match token.m.lock() {
            Ok(m) => {
                let timeout = timeout_value_in_context(&ctx);
                let record = Record::Request {
                    id: index,
                    ctx,
                    oid,
                    req,
                };
                if let Err(e) = tx.send(record).await {
                    Err(e.to_string().into())
                } else {
                    match token.cv.wait_timeout(m, Duration::from_millis(timeout)) {
                        Ok(mut r) => {
                            if r.1.timed_out() {
                                Err("timed_out.".into())
                            } else {
                                r.0.take().unwrap()
                            }
                        }
                        Err(e) => Err(e.to_string().into()),
                    }
                }
            }
            Err(e) => Err(e.to_string().into()),
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
                    let mut g = token.m.lock().unwrap();
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
    pub fn connect_to(self, addr: String) -> JoinHandle<()> {
        let h = task::spawn(async {
            let r = self.run(addr).await;
            info!("terminal run result: {:?}", r);
        });
        std::thread::sleep(Duration::from_secs(1));
        h
    }
    async fn run(self, addr: impl ToSocketAddrs) -> std::io::Result<()> {
        #[derive(Debug)]
        enum SelectedValue {
            ReadNone,
            WriteNone,
            Tick,
            Read(Record),
            Write(Record),
        }

        let stream = TcpStream::connect(addr).await?;
        info!("connected to {}", stream.peer_addr()?);
        let (reader, writer) = (&stream, &stream);
        let read_framed = FramedRead::new(reader, RecordCodec::<u32, Record>::default());
        let mut write_framed = FramedWrite::new(writer, RecordCodec::<u32, Record>::default());

        let (tx, rx) = unbounded();
        self.set_tx(Some(tx)).await;
        let _terminal_clean = DropGuard::new(self.clone(), |t| {
            task::block_on(async move {
                info!("terminal quit.");
                t.clean().await;
            });
        });

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
        dbg!(&now);
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
