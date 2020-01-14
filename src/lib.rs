// -- libr.rs --

#[macro_use]
extern crate lazy_static;
extern crate async_std;
extern crate bincode;
extern crate futures;
extern crate futures_codec;
extern crate serde;
extern crate servant_macro;
extern crate servant_codec as codec;
extern crate servant_log;

// --

#[cfg(feature = "invoke")]
pub use servant_macro::invoke_interface;
#[cfg(feature = "notify")]
pub use servant_macro::notify_interface;
#[cfg(feature = "query")]
pub use servant_macro::query_interface;
#[cfg(feature = "report")]
pub use servant_macro::report_interface;
// pub use servant_macro::invoke_interface_with_persistency;

mod list;
mod servant;
mod factory;
mod freeze;
mod db;

#[cfg(any(feature = "adapter", feature = "terminal"))]
mod drop_guard;

#[cfg(feature = "adapter")]
mod accept;
#[cfg(feature = "adapter")]
mod adapter;

#[cfg(feature = "terminal")]
mod terminal;

#[cfg(feature = "default_gateway")]
mod gateway;

// --

pub use crate::servant::{
    Context, NotifyServant, Oid, QueryServant, ReportServant, Servant, ServantError, ServantRegister,
    ServantResult,
};

pub use factory::{FactoryEntry, FactoryServant, FactoryProxy};

#[cfg(feature = "adapter")]
pub use {
    accept::accept_on,
    adapter::{Adapter, AdapterRegister},
};

#[cfg(feature = "terminal")]
pub use terminal::Terminal;

#[cfg(feature = "default_gateway")]
pub use gateway::GatewayProxy;
