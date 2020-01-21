// -- gateway.rs --

use {
    crate::{
        self as servant,
        servant::{Oid, ServantRegister, UserCookie},
        task,
    },
};

// --

#[servant_macro::watch_interface]
pub trait Gateway {
    fn export_servants(&self) -> Vec<Oid>;
    fn export_report_servants(&self) -> Vec<Oid>;
    fn login(&self, name: String, password: String) -> UserCookie;
    fn version(&self) -> String;
}

// --

pub struct GatewayEntity {
    sr: ServantRegister,
}
impl GatewayEntity {
    pub fn new(sr: ServantRegister) -> Self {
        Self { sr }
    }
}

impl Gateway for GatewayEntity {
    fn export_servants(&self) -> Vec<Oid> {
        task::block_on(async {
            self.sr.export_servants().await
        })
    }
    fn export_report_servants(&self) -> Vec<Oid> {
        task::block_on(async {
            self.sr.export_report_servants().await
        })
    }
    fn login(&self, _name: String, _password: String) -> UserCookie {
        238
    }
    fn version(&self) -> String {
        "Version 1.0".to_string()
    }
}
