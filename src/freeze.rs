// -- freeze.rs --

use {
    crate::servant::{Oid, ServantResult, ServantEntry},
    std::collections::HashMap,
    log::{warn}
};

// --

pub struct Freeze {
    map: HashMap<String, Box<dyn Fn(&str, &[u8]) -> ServantEntry + Send>>,
    db: HashMap<Oid, Vec<u8>>,
}

impl Freeze {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            db: HashMap::new(),
        }
    }
    fn store_into_db(&mut self, oid: &Oid, bytes: &[u8]) -> ServantResult<()> {
        self.db.insert(oid.clone(), bytes.to_vec());
        dbg!(oid);
        Ok(())
    }
    fn load_from_db(&self, oid: &Oid) -> ServantResult<Vec<u8>> {
        dbg!(oid);
        if let Some(v) = self.db.get(oid) {
            Ok(v.clone())
        } else {
            Err(format!("{} dosen't exist in db.", oid).into())
        }
    }
    pub fn register<F>(&mut self, category: &str, f: F) -> ServantResult<()>
    where
        F: Fn(&str, &[u8]) -> ServantEntry + 'static + Send,
    {
        if self.map.get(category).is_none() {
            self.map.insert(category.to_string(), Box::new(f));
            Ok(())
        } else {
            Err(format!("category: {} is duplicate in freeze.", category).into())
        }
    }
    pub fn store(&mut self, _oid: &Oid, bytes: &[u8]) -> ServantResult<()> {
        self.store_into_db(_oid, bytes)
    }
    pub fn load(&self, oid: &Oid) -> Option<ServantEntry> {
        let category = oid.category();
        match self.load_from_db(oid) {
            Ok(bytes) => {
                if let Some(f) = self.map.get(&category.to_string()) {
                    Some(f(oid.name(), &bytes))
                } else {
                    warn!("category: {}, create fn dosen't exist in freeze.", category);
                    None
                }
            }
            Err(e) => {
                warn!("laod_from_db({}) error({})", oid, e.to_string());
                None
            }
        }
    }
}