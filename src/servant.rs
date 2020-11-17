// -- servant.rs --

use {
    crate::utilities::BoolToOption,
    serde::{Deserialize, Serialize},
    std::{collections::HashMap, error::Error, net::SocketAddr},
};

// --

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServantError {
    NoSupportSerializable,
    DuplicateOid,
    Other(String),
}

impl Error for ServantError {}

impl<T: Into<String>> std::convert::From<T> for ServantError {
    fn from(e: T) -> Self {
        Self::Other(e.into())
    }
}

impl std::fmt::Display for ServantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ServantError({:?})", self)
    }
}

pub type ServantResult<T> = Result<T, ServantError>;

// --

#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Oid {
    name: String,
    category: String,
}

impl Oid {
    pub fn new(name: &str, category: &str) -> Self {
        Self {
            name: name.to_string(),
            category: category.to_string(),
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn category(&self) -> &str {
        &self.category
    }
}

impl std::fmt::Display for Oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Oid({}: {})", self.name, self.category)
    }
}

// --

pub type UserCookie = usize;
pub type ConnectionId = SocketAddr;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Context {
    pub timeout_millisecond: Option<u64>,
    pub user_cookie: Option<UserCookie>,
    pub connection_id: Option<ConnectionId>,
    pub attributes: HashMap<String, String>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            timeout_millisecond: None,
            user_cookie: None,
            connection_id: None,
            attributes: HashMap::new(),
        }
    }
}

// --

cfg_server! {
    use crate::{
        freeze::{Freeze, MemoryDb},
        utilities::{List, Pointer},
        sync::{Arc, Mutex},
    };

    pub(crate) type ServantEntity = Arc<Mutex<Box<dyn Servant + Send>>>;
    pub(crate) type ReportServantEntity = Arc<Mutex<Box<dyn ReportServant + Send>>>;
    pub(crate) type WatchServantEntity = Arc<Mutex<Box<dyn WatchServant + Send>>>;

    #[derive(Clone)]
    struct ServantRecord {
        servant: ServantEntity,
        node: Option<Pointer<Oid>>,
    }

    struct EvictorList {
        max_count: usize,
        list: List<Oid>,
    }
    impl EvictorList {
        fn new(count: usize) -> Self {
            Self {
                max_count: count,
                list: List::<Oid>::new(20),
            }
        }
        fn push(&mut self, oid: &Oid) -> (Pointer<Oid>, Option<Oid>) {
            if self.list.len() >= self.max_count {
                (self.list.push(oid.clone()), self.list.pop())
            } else {
                (self.list.push(oid.clone()), None)
            }
        }
        fn top(&mut self, p: &Pointer<Oid>) {
            self.list.top(p)
        }
        fn to_vec(&self) -> Vec<Oid> {
            self.list.to_vec()
        }
    }

    struct _ServantRegister {
        servants: HashMap<Oid, ServantRecord>,
        report_servants: HashMap<Oid, ReportServantEntity>,
        watch: Option<WatchServantEntity>,
        evictor: EvictorList,
        freeze: Freeze,
    }

    #[derive(Clone)]
    pub struct ServantRegister(Arc<Mutex<_ServantRegister>>);
    impl ServantRegister {
        pub fn new(max_count_of_evictor_list: usize) -> Self {
            Self(Arc::new(Mutex::new(_ServantRegister {
                servants: HashMap::new(),
                report_servants: HashMap::new(),
                watch: None,
                evictor: EvictorList::new(max_count_of_evictor_list),
                freeze: Freeze::new(Box::new(MemoryDb::new())),
            })))
        }
        pub(crate) async fn evictor_to_vec(&self) -> Vec<Oid> {
            let g = self.0.lock().await;
            g.evictor.to_vec()
        }
        pub async fn set_watch_servant(&self, watch: WatchServantEntity) -> Option<WatchServantEntity> {
            let mut g = self.0.lock().await;
            g.watch.replace(watch)
        }
        pub(crate) async fn watch_servant(&self) -> Option<WatchServantEntity> {
            let g = self.0.lock().await;
            g.watch.as_ref().map(Clone::clone)
        }
        pub(crate) async fn servants(&self) -> Vec<Oid> {
            let g = self.0.lock().await;
            g.servants.keys().map(|v| v.clone()).collect()
        }
        pub(crate) async fn find_servant(&self, oid: &Oid) -> Option<ServantEntity> {
            if let Some((servant, record)) = {
                let mut g = self.0.lock().await;
                if let Some(r) = g.servants.get(&oid).map(|s| s.clone()) {
                    r.node.and_then(|v| Some(g.evictor.top(&v)));
                    return Some(r.servant);
                }
                g.freeze.load(oid).map(|s| {
                    let (node, abandoner_oid) = g.evictor.push(&oid);
                    g.servants.insert(
                        oid.clone(),
                        ServantRecord {
                            servant: s.clone(),
                            node: Some(node),
                        },
                    );
                    (s, abandoner_oid.and_then(|a| Some((a.clone(), g.servants.remove(&a).unwrap()))))
                })
            }
            {
                if let Some((oid, r)) = record {
                    let v: Vec<u8> = r.servant.lock().await.dump().unwrap();
                    self.0.lock().await.freeze.store(&oid, &v).unwrap();
                }
                Some(servant)
            } else {
                None
            }
        }
        pub(crate) async fn report_servants(&self) -> Vec<Oid> {
            let g = self.0.lock().await;
            g.report_servants.keys().map(|v| v.clone()).collect()
        }
        pub(crate) async fn find_report_servant(&self, oid: &Oid) -> Option<ReportServantEntity> {
            let g = self.0.lock().await;
            g.report_servants.get(&oid).map(|s| s.clone())
        }
        pub async fn add_servant(&self, category: &str, entity: ServantEntity) -> ServantResult<()> {
            let (oid, serializable) = {
                let g = entity.lock().await;
                (
                    Oid::new(g.name(), category),
                    !(g.dump() == Err(ServantError::NoSupportSerializable)),
                )
            };
            let mut g = self.0.lock().await;
            if g.servants.get(&oid).is_some() {
                Err(ServantError::DuplicateOid)?;
            }
            let node = serializable.then2(|| {
                let (node, _) = g.evictor.push(&oid);
                node
            });
            g.servants.insert(
                oid.clone(),
                ServantRecord {
                    servant: entity,
                    node,
                },
            );
            Ok(())
        }
        pub async fn add_report_servant(
            &self,
            category: &str,
            entity: ReportServantEntity,
        ) -> Option<ReportServantEntity> {
            let oid = {
                let g = entity.lock().await;
                Oid::new(g.name(), category)
            };
            let mut g = self.0.lock().await;
            g.report_servants.insert(oid, entity)
        }
        pub async fn enroll_in_freeze<F>(&self, category: &str, f: F) -> ServantResult<()>
        where
            F: Fn(&str, &[u8]) -> ServantEntity + 'static + Send,
        {
            let mut g = self.0.lock().await;
            g.freeze.enroll(category, f)
        }
    }
}

// --

pub trait Servant {
    fn name(&self) -> &str;
    fn dump(&self) -> ServantResult<Vec<u8>> {
        Err(ServantError::NoSupportSerializable)
    }
    fn serve(&mut self, ctx: Option<Context>, req: Vec<u8>) -> Vec<u8>;
}

pub trait WatchServant {
    fn serve(&mut self, req: Vec<u8>) -> Vec<u8>;
}

pub trait ReportServant {
    fn name(&self) -> &str;
    fn serve(&mut self, req: Vec<u8>);
}

pub trait NotifyServant {
    fn serve(&mut self, req: Vec<u8>);
}

// --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Record {
    Notice {
        id: usize,
        msg: Vec<u8>,
    },
    Report {
        id: usize,
        oid: Oid,
        msg: Vec<u8>,
    },
    Request {
        id: usize,
        ctx: Option<Context>,
        oid: Option<Oid>,
        req: Vec<u8>,
    },
    Response {
        id: usize,
        oid: Option<Oid>,
        ret: Vec<u8>,
    },
}

impl Default for Record {
    fn default() -> Self {
        Self::Notice {
            id: 0,
            msg: Vec::new(),
        }
    }
}
