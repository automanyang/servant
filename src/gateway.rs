// -- gateway.rs --

use {
    crate::{
        self as servant,
        adapter::AdapterRegister,
        servant::{Context, Oid, ServantRegister},
    },
    async_std::task,
};

// --

#[servant_macro::query_interface]
pub trait Gateway {
    fn export_servants(&self) -> Vec<Oid>;
    fn export_report_servants(&self) -> Vec<Oid>;
    fn shutdown(&self, passcode: usize);
    fn login(&self, name: String, password: String) -> Context;
}

// --

pub struct GatewayEntry;

impl Gateway for GatewayEntry {
    fn export_servants(&self) -> Vec<Oid> {
        ServantRegister::instance().export_servants()
    }
    fn export_report_servants(&self) -> Vec<Oid> {
        ServantRegister::instance().export_report_servants()
    }
    fn shutdown(&self, passcode: usize) {
        task::block_on(async {
            AdapterRegister::instance().clean(passcode).await;
        });
    }
    fn login(&self, _name: String, _password: String) -> Context {
        Context::new()
    }
}
