// -- server.rs --

use {
    crate::{
        adapter::{Adapter, AdapterRegister},
        config,
        servant::ServantRegister,
    },
    async_std::{
        net::{TcpListener, TcpStream, ToSocketAddrs},
        prelude::*,
        task,
    },
    futures::{channel::mpsc::unbounded, pin_mut, select, FutureExt as _},
    log::info,
};

// --

pub struct Server<T> {
    config: config::Server,
    sr: ServantRegister,
    ar: AdapterRegister,
    notifier: Option<T>,
}

impl<T: Clone> Server<T> {
    pub fn new() -> Self {
        let config = config::Server::load();
        let sr = ServantRegister::new(config.max_count_of_evictor_list);
        let ar = AdapterRegister::new();
        Self {
            config,
            sr,
            ar,
            notifier: None,
        }
    }
    pub fn set_notifier<F>(&mut self, f: F) -> Option<T>
    where
        F: 'static + Fn(AdapterRegister) -> T,
    {
        let ar = self.ar.clone();
        self.notifier.replace(f(ar))
    }
    pub fn notifier(&self) -> Option<T> {
        self.notifier.as_ref().map(|x| x.clone())
    }
    pub fn servant_register(&self) -> ServantRegister {
        self.sr.clone()
    }
    pub fn adapter_register(&self) -> AdapterRegister {
        self.ar.clone()
    }
    pub async fn accept_on(self, addr: impl ToSocketAddrs) -> std::io::Result<()> {
        #[derive(Debug)]
        enum SelectedValue {
            RxNone,
            IncomingNone,
            Incoming(TcpStream),
        };
        let serve_count = self.config.serve_count_by_adapter;
        let (tx, rx) = unbounded();
        self.ar.set_accept(tx).await;
        let listener = TcpListener::bind(addr).await?;
        let incoming = listener.incoming();
        pin_mut!(incoming, rx);
        loop {
            let value = select! {
                connection_stream = incoming.next().fuse() => match connection_stream {
                    Some(stream) => SelectedValue::Incoming(stream?),
                    None => SelectedValue::IncomingNone,
                },
                from_rx = rx.next().fuse() => match from_rx {
                    Some(record) => unreachable!(),
                    None => SelectedValue::RxNone,
                },
            };
            match value {
                SelectedValue::Incoming(stream) => {
                    info!("Accepting from: {}", stream.peer_addr()?);
                    let adapter = Adapter::new(self.ar.clone(), self.sr.clone(), serve_count);
                    let _handle = task::spawn(adapter.run(stream));
                }
                _ => {
                    info!("accept loop break due to {:?}", value);
                    break;
                }
            }
        }
        Ok(())
    }
}
