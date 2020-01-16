// -- servant.rs --

use {
    crate::{
        freeze::{Freeze, MemoryDb},
        utilities::{List, Pointer},
        config::Config,
    },
    serde::{Deserialize, Serialize},
    std::sync::{Arc, Mutex, MutexGuard},
    std::{collections::HashMap, error::Error, net::SocketAddr},
};

// --

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServantError {
    NoSupportSerializable,
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
type ConnectionId = SocketAddr;

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

lazy_static! {
    static ref REGISTER: ServantRegister = ServantRegister({
        let max_count_of_evictor_list = Config::instance().max_count_of_evictor_list;
        Mutex::new(_ServantRegister {
            servants: HashMap::new(),
            report_servants: HashMap::new(),
            #[cfg(feature = "default_gateway")]
            query: Some(Arc::new(Mutex::new(super::gateway::GatewayServant::new(
                super::gateway::GatewayEntry,
            )))),
            #[cfg(not(feature = "default_gateway"))]
            query: None,
            evictor: List::new(max_count_of_evictor_list),
            freeze: Freeze::new(Box::new(MemoryDb::new())),
        })
    });
}

pub(crate) type ServantEntry = Arc<Mutex<dyn Servant + Send>>;
type ReportServantEntry = Arc<Mutex<dyn ReportServant + Send>>;
type QueryServantEntry = Arc<Mutex<dyn QueryServant + Send>>;

#[derive(Clone)]
struct ServantRecord {
    servant: ServantEntry,
    node: Option<Pointer<Oid>>,
}

struct _ServantRegister {
    servants: HashMap<Oid, ServantRecord>,
    report_servants: HashMap<Oid, ReportServantEntry>,
    query: Option<QueryServantEntry>,
    evictor: List<Oid>,
    freeze: Freeze,
}

pub struct ServantRegister(Mutex<_ServantRegister>);
impl ServantRegister {
    pub fn instance() -> &'static Self {
        &REGISTER
    }
    pub fn set_query_servant(&self, query: QueryServantEntry) {
        let mut g = self.0.lock().unwrap();
        g.query.replace(query);
    }
    pub fn query_servant(&self) -> Option<QueryServantEntry> {
        let g = self.0.lock().unwrap();
        g.query.as_ref().map(Clone::clone)
    }
    pub fn find_servant(&self, oid: &Oid) -> Option<ServantEntry> {
        let mut g = self.0.lock().unwrap();
        if let Some(r) = g.servants.get(&oid).map(|s| s.clone()) {
            if let Some(node) = r.node {
                g.evictor.top(node);
            }
            return Some(r.servant);
        }
        if let Some(s) = g.freeze.load(oid) {
            evict_last_one(&mut g);
            let node = Some(g.evictor.push(oid.clone()));
            if let Some(_) = g.servants.insert(
                oid.clone(),
                ServantRecord {
                    servant: s.clone(),
                    node,
                },
            ) {
                assert!(false, "{} duplicate.", oid);
            }
            Some(s)
        } else {
            None
        }
    }
    pub fn find_report_servant(&self, oid: &Oid) -> Option<ReportServantEntry> {
        let g = self.0.lock().unwrap();
        g.report_servants.get(&oid).map(|s| s.clone())
    }
    pub fn add_servant(&self, category: &str, entity: ServantEntry) {
        let (oid, serializable) = {
            let g = entity.lock().unwrap();
            (
                Oid::new(g.name(), category),
                !(g.dump() == Err(ServantError::NoSupportSerializable)),
            )
        };
        let mut g = self.0.lock().unwrap();
        let node = if serializable {
            evict_last_one(&mut g);
            Some(g.evictor.push(oid.clone()))
        } else {
            None
        };
        g.servants.insert(
            oid,
            ServantRecord {
                servant: entity,
                node,
            },
        );
    }
    pub fn add_report_servant(&self, category: &str, entry: ReportServantEntry) {
        let oid = {
            let g = entry.lock().unwrap();
            Oid::new(g.name(), category)
        };
        let mut g = self.0.lock().unwrap();
        g.report_servants.insert(oid, entry);
    }
    pub fn export_servants(&self) -> Vec<Oid> {
        let g = self.0.lock().unwrap();
        g.servants.keys().map(|x| x.clone()).collect()
    }
    pub fn export_report_servants(&self) -> Vec<Oid> {
        let g = self.0.lock().unwrap();
        g.report_servants.keys().map(|x| x.clone()).collect()
    }
    pub fn enroll_in_freeze<F>(&self, category: &str, f: F) -> ServantResult<()>
    where F: Fn(&str, &[u8]) -> ServantEntry + 'static + Send, {
        let mut g = self.0.lock().unwrap();
        g.freeze.enroll(category, f)
    }
}

#[allow(unused)]
fn evict_all(mg: &mut MutexGuard<'_, _ServantRegister>) {
    while let Some(abandoner) = mg.evictor.pop() {
        if let Some(r) = mg.servants.remove(&abandoner) {
            let v: Vec<u8> = { r.servant.lock().unwrap().dump().unwrap() };
            mg.freeze.store(&abandoner, &v).unwrap();
        }
    }
}

fn evict_last_one(mg: &mut MutexGuard<'_, _ServantRegister>) {
    if let Some(abandoner) = mg.evictor.evict() {
        if let Some(r) = mg.servants.remove(&abandoner) {
            let v: Vec<u8> = { r.servant.lock().unwrap().dump().unwrap() };
            mg.freeze.store(&abandoner, &v).unwrap();
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

pub trait QueryServant {
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
