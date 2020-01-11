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
    map: HashMap<String, Box<dyn Fn(String) -> ServantEntry + Send>>,
}

impl FactoryEntry {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn registe<F>(&mut self, category: String, f: F) -> ServantResult<()>
    where
        F: Fn(String) -> ServantEntry + 'static + Send,
    {
        if self.map.get(&category).is_none() {
            self.map.insert(category, Box::new(f));
            Ok(())
        } else {
            Err(format!("category: {}, category is duplicate in factory.", category).into())
        }
    }
}

impl Factory for FactoryEntry {
    fn create(&self, _ctx: Option<Context>, name: String, category: String) -> ServantResult<Oid> {
        if let Some(f) = self.map.get(&category) {
            let entity = f(name.clone());
            ServantRegister::instance().add_servant(&category, entity);
            Ok(Oid::new(&name, &category))
        } else {
            Err(format!(
                "{}, create fn dosen't exist in factory.",
                Oid::new(&name, &category)
            )
            .into())
        }
    }
}
