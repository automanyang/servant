// -- libr.rs --

// #[macro_use]
// extern crate lazy_static;
extern crate async_std;
extern crate bincode;
extern crate futures;
extern crate futures_codec;
extern crate serde;
extern crate servant_codec as codec;
extern crate servant_log;
extern crate servant_macro;

// --

#[cfg(feature = "invoke")]
pub use servant_macro::invoke_interface;
#[cfg(feature = "notify")]
pub use servant_macro::notify_interface;
#[cfg(feature = "report")]
pub use servant_macro::report_interface;
#[cfg(feature = "watch")]
pub use servant_macro::watch_interface;

mod config;
mod db;
mod freeze;
mod servant;
mod utilities;

#[cfg(feature = "adapter")]
mod adapter;
#[cfg(feature = "adapter")]
mod server;

#[cfg(feature = "terminal")]
mod client;
#[cfg(feature = "terminal")]
mod terminal;

#[cfg(feature = "gateway_entity")]
mod gateway;

#[cfg(feature = "factory_entity")]
mod factory;

#[cfg(feature = "admin_entity")]
mod admin;

// --

pub use crate::servant::{
    Context, NotifyServant, Oid, ReportServant, Servant, ServantError, ServantRegister,
    ServantResult, UserCookie, WatchServant,
};

#[cfg(feature = "adapter")]
pub use {adapter::AdapterRegister, server::Server};

#[cfg(feature = "terminal")]
pub use {client::Client, terminal::Terminal};

#[cfg(all(feature = "gateway_entity", feature = "terminal"))]
pub use gateway::GatewayProxy;
#[cfg(all(feature = "gateway_entity", feature = "adapter"))]
pub use gateway::{GatewayEntity, GatewayServant};

#[cfg(all(feature = "admin_entity", feature = "terminal"))]
pub use factory::FactoryProxy;
#[cfg(all(feature = "factory_entity", feature = "adapter"))]
pub use factory::{FactoryEntity, FactoryServant};

#[cfg(all(feature = "admin_entity", feature = "terminal"))]
pub use admin::AdminProxy;
#[cfg(all(feature = "admin_entity", feature = "adapter"))]
pub use admin::{AdminEntity, AdminServant};
