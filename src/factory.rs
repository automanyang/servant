// -- factory.rs --

use {
    crate::{
        self as servant,
        servant::{Context, Oid, ServantEntry, ServantRegister, ServantResult},
    },
    std::collections::HashMap,
};

// --

#[servant::invoke_interface]
pub trait Factory {
    fn create(&self, name: String, category: String) -> ServantResult<Oid>;
}

// --

// #[derive(serde::Serialize, serde::Deserialize)]
pub struct FactoryEntry {
    map: HashMap<String, Box<dyn Fn(&str) -> ServantEntry + Send>>,
}

impl FactoryEntry {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn enroll<F>(&mut self, category: &str, f: F) -> ServantResult<()>
    where
        F: Fn(&str) -> ServantEntry + 'static + Send,
    {
        if self.map.get(&category.to_string()).is_none() {
            self.map.insert(category.to_string(), Box::new(f));
            Ok(())
        } else {
            Err(format!("category: {}, category is duplicate in factory.", category).into())
        }
    }
}

impl Factory for FactoryEntry {
    fn create(&self, _ctx: Option<Context>, name: String, category: String) -> ServantResult<Oid> {
        let oid = Oid::new(&name, &category);
        if let Some(f) = self.map.get(&category) {
            let entity = f(&name);
            ServantRegister::instance().add_servant(&category, entity);
            Ok(oid)
        } else {
            Err(format!(
                "{}, create fn dosen't exist in factory.",
                oid
            )
            .into())
        }
    }
}
