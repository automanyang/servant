// -- factory.rs --

use {
    crate::{
        self as servant,
        servant::{Context, Oid, ServantEntity, ServantRegister, ServantResult},
        task
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
pub struct FactoryEntity {
    sr: ServantRegister,
    map: HashMap<String, Box<dyn Fn(&str) -> ServantEntity + Send>>,
}

impl FactoryEntity {
    pub fn new(sr: ServantRegister) -> Self {
        Self {
            sr,
            map: HashMap::new(),
        }
    }
    pub fn enroll<F>(&mut self, category: &str, f: F) -> ServantResult<()>
    where
        F: Fn(&str) -> ServantEntity + 'static + Send,
    {
        if self.map.get(&category.to_string()).is_none() {
            self.map.insert(category.to_string(), Box::new(f));
            Ok(())
        } else {
            Err(format!("category: {}, category is duplicate in factory.", category).into())
        }
    }
}

impl Factory for FactoryEntity {
    fn create(&self, _ctx: Option<Context>, name: String, category: String) -> ServantResult<Oid> {
        let oid = Oid::new(&name, &category);
        if let Some(f) = self.map.get(&category) {
            task::block_on(async {
                let entity = f(&name);
                self.sr.add_servant(&category, entity).await;
            });
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
