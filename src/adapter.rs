// -- adapter.rs --

use {
    super::{
        utilities::{DropGuard},
        servant::{Record, ServantRegister, ServantResult},
        sync::{Arc, Mutex, Semaphore, SemaphoreGuard}, task
    },
    async_std::{net::TcpStream, prelude::*, },
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
        net::SocketAddr,
    },
};

// --

struct _Register {
    passcode: usize,
    id: usize,
    accept_tx: Option<UnboundedSender<()>>,
    senders: HashMap<SocketAddr, UnboundedSender<Record>>,
}

#[derive(Clone)]
pub struct AdapterRegister(Arc<Mutex<_Register>>);
impl AdapterRegister {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(_Register {
            passcode: 238,
            id: 0,
            accept_tx: None,
            senders: HashMap::new(),
        })))
    }
    pub async fn clean(&self, passcode: usize) {
        let mut g = self.0.lock().await;
        if passcode == g.passcode {
            g.accept_tx.take();
            g.senders.clear();
        }
    }
    pub(crate) async fn set_accept(&self, tx: UnboundedSender<()>) {
        let mut g = self.0.lock().await;
        g.accept_tx = Some(tx);
    }
    pub(crate) async fn insert(&self, addr: SocketAddr, tx: UnboundedSender<Record>) {
        let mut g = self.0.lock().await;
        g.senders.insert(addr, tx);
    }
    pub(crate) async fn remove(&self, addr: &SocketAddr) {
        let mut g = self.0.lock().await;
        g.senders.remove(addr);
    }
    pub async fn send(&self, msg: Vec<u8>) {
        let mut g = self.0.lock().await;
        g.id += 1;
        let notice = Record::Notice { id: g.id, msg };
        let mut values = g.senders.values();
        while let Some(mut s) = values.next() {
            s.send(notice.clone())
                .await
                .unwrap_or_else(|e| warn!("{}", e.to_string()));
        }
    }
}

// --

pub(crate) struct Adapter {
    sr: ServantRegister,
    ar: AdapterRegister,
    max_serve_count: usize,
}

impl Adapter {
    pub(crate) fn new(ar: AdapterRegister, sr: ServantRegister, max_serve_count: usize) -> Self {
        Self {
            sr,
            ar,
            max_serve_count,
        }
    }
    pub(crate) async fn run(self, stream: TcpStream) -> std::io::Result<()> {
        #[derive(Debug)]
        enum SelectedValue {
            ReadNone,
            WriteNone,
            Read(Record),
            Write(Record),
        };

        let addr = stream.peer_addr().unwrap();
        info!("connected from {}", &addr);
        let (reader, writer) = (&stream, &stream);
        let read_framed = FramedRead::new(reader, RecordCodec::<u32, Record>::default());
        let mut write_framed = FramedWrite::new(writer, RecordCodec::<u32, Record>::default());

        let sem = Semaphore::new(self.max_serve_count);
        let (tx, rx) = unbounded();
        self.ar.insert(addr, tx.clone()).await;

        let _adapter_clean = DropGuard::new((addr, self.ar.clone()), |(a, ar)| {
            task::block_on(async move {
                info!("adapter from {} quit.", &addr);
                ar.remove(&a).await;
            });
        });

        pin_mut!(read_framed, rx);
        loop {
            let value = select! {
                from_terminal = read_framed.next().fuse() => match from_terminal {
                    Some(record) => SelectedValue::Read(record?),
                    None => SelectedValue::ReadNone,
                },
                to_terminal = rx.next().fuse() => match to_terminal {
                    Some(record) => SelectedValue::Write(record),
                    None => SelectedValue::WriteNone,
                },
            };

            match value {
                SelectedValue::Read(record) => {
                    let g = sem.lock().await;
                    let tx2 = tx.clone();
                    let sr = self.sr.clone();
                    task::spawn(serve(g, sr, tx2, record));
                }
                SelectedValue::Write(record) => {
                    write_framed.send(record).await?;
                }
                _ => {
                    info!(
                        "loop break due to SelectValue({:?}). peer address: {}",
                        &value, &addr
                    );
                    break;
                }
            }
        }

        Ok(())
    }
}

async fn serve(_g: SemaphoreGuard, sr: ServantRegister, mut tx: UnboundedSender<Record>, record: Record) {
    match record {
        Record::Report { id, oid, msg } => {
            let _id = id;
            if let Some(servant) = sr.find_report_servant(&oid).await {
                servant.lock().await.serve(msg);
            } else {
                warn!("{} dosen't exist.", &oid);
            }
        }
        Record::Request { id, ctx, oid, req } => {
            let ret: ServantResult<Vec<u8>> = if let Some(oid) = &oid {
                if let Some(servant) = sr.find_servant(oid).await {
                    Ok(servant.lock().await.serve(ctx, req))
                } else {
                    Err(format!("{} dosen't exist.", &oid).into())
                }
            } else {
                if let Some(watch) = sr.watch_servant().await {
                    let mut q = watch.lock().await;
                    Ok(q.serve(req))
                } else {
                    Err("watch servant dosen't exist.".into())
                }
            };
            match bincode::serialize(&ret) {
                Ok(ret) => {
                    let record = Record::Response { id, oid, ret };
                    if let Err(e) = tx.send(record).await {
                        warn!("{}", e.to_string());
                    }
                }
                Err(e) => warn!("{}", e.to_string()),
            }
        }
        Record::Response { .. } => unreachable!(),
        Record::Notice { .. } => unreachable!(),
    };
}
