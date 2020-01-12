// -- sqlite.rs --

use {
    crate::{
        freeze::Storage,
        servant::{Oid, ServantResult}
    },
};

// --

pub struct SqliteDb;

impl SqliteDb {
    #[allow(unused)]
    pub fn new(_fn: &str) -> Self {
        Self
    }
}

impl Storage for SqliteDb {
    fn store(&mut self, _oid: &Oid, _bytes: &[u8]) -> ServantResult<()> {
        unimplemented!();
    }
    fn load(&mut self, _oid: &Oid) -> ServantResult<Vec<u8>> {
        unimplemented!();
    }
}