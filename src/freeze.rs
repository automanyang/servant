// -- freeze.rs --

use {
    crate::servant::{Oid, ServantResult, ServantEntity},
    std::collections::HashMap,
};

// --

pub trait Storage {
    fn store(&mut self, oid: &Oid, bytes: &[u8]) -> ServantResult<()>;
    fn load(&mut self, oid: &Oid) -> ServantResult<Vec<u8>>;
}

// --

pub struct MemoryDb(HashMap<Oid, Vec<u8>>);
impl MemoryDb {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl Storage for MemoryDb {
    fn store(&mut self, oid: &Oid, bytes: &[u8]) -> ServantResult<()> {
        self.0.insert(oid.clone(), bytes.to_vec());
        Ok(())
    }
    fn load(&mut self, oid: &Oid) -> ServantResult<Vec<u8>> {
        self.0.remove(oid).ok_or(format!("{} dosen't exist in db.", oid).into())
    }
}

// --

pub struct Freeze {
    map: HashMap<String, Box<dyn Fn(&str, &[u8]) -> ServantEntity + Send>>,
    db: Box<dyn Storage + Send>,
}

impl Freeze {
    pub fn new(db: Box<dyn Storage + Send>) -> Self {
        Self {
            map: HashMap::new(),
            db
        }
    }
    pub fn enroll<F>(&mut self, category: &str, f: F) -> ServantResult<()>
    where
        F: Fn(&str, &[u8]) -> ServantEntity + 'static + Send,
    {
        if self.map.get(category).is_none() {
            self.map.insert(category.to_string(), Box::new(f));
            Ok(())
        } else {
            Err(format!("category: {} is duplicate in freeze.", category).into())
        }
    }
    pub fn store(&mut self, oid: &Oid, bytes: &[u8]) -> ServantResult<()> {
        self.db.store(oid, bytes)
    }
    pub fn load(&mut self, oid: &Oid) -> Option<ServantEntity> {
        let category = oid.category();
        match self.db.load(oid) {
            Ok(bytes) => {
                if let Some(f) = self.map.get(&category.to_string()) {
                    Some(f(oid.name(), &bytes))
                } else {
                    log::warn!("category: {}, create fn dosen't exist in freeze.", category);
                    None
                }
            }
            Err(e) => {
                log::warn!("laod_from_db({}) error({})", oid, e.to_string());
                None
            }
        }
    }
}