// -- help.rs --

use crate::{
    self as servant,
    config::HelpData,
};

// --

#[servant_macro::watch_interface]
pub trait Help {
    fn about(&self) -> String;
    fn readme(&self) -> String;
    fn version(&self) -> String;
    fn list(&self) -> Vec<String>;
    fn help(&self, key: String) -> Option<String>;
}

pub(crate) struct HelpEntity {
    h: HelpData,
}
impl HelpEntity {
    pub(crate) fn new(h: &HelpData) -> Self {
        Self { h: h.clone() }
    }
}
impl Help for HelpEntity {
    fn about(&self) -> String {
        self.h.about.clone()
    }
    fn readme(&self) -> String {
        self.h.readme.clone()
    }
    fn version(&self) -> String {
        self.h.version.clone()
    }
    fn list(&self) -> Vec<String> {
        self.h.context.keys().map(|v| v.clone()).collect()
    }
    fn help(&self, key: String) -> Option<String> {
        self.h.context.get(&key).map(|v| v.clone())
    }
}
