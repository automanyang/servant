// -- libr.rs --

#[macro_use]
mod macros;
#[macro_use]
mod utilities;

// extern crate async_std;
// extern crate bincode;
// extern crate futures;
// extern crate futures_codec;
// extern crate serde;
extern crate servant_codec as codec;
// extern crate servant_log;

// --

pub use utilities::*;

cfg_server_or_client! {
    extern crate servant_macro;
    pub use servant_macro::invoke_interface;
    pub use servant_macro::watch_interface;
    pub use servant_macro::report_interface;
    pub use servant_macro::notify_interface;

    mod config;
    mod servant;
    mod sync;
    mod task;

    pub use crate::servant::{
        Context, NotifyServant, Oid, ReportServant, Servant, ServantError,
        ServantResult, UserCookie, WatchServant,
    };
}

// --

cfg_server! {
    mod db;
    mod freeze;
    mod adapter;
    mod server;
    pub use {adapter::AdapterRegister, server::Server, crate:: servant::ServantRegister};
}

cfg_client! {
    mod client;
    mod terminal;
    pub use {client::Client, terminal::Terminal};
}

cfg_help_entity! {
    mod help;
    pub use help::{HelpProxy, HelpServant};
}

cfg_factory_entity! {
    mod factory;
    pub use factory::{FactoryProxy, FactoryEntity, FactoryServant};
}

cfg_admin_entity! {
    mod admin;
    pub use admin::{AdminProxy, AdminEntity, AdminServant};
}
