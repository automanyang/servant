// -- macro.rs --

#![allow(unused_macros)]

// --

macro_rules! cfg_server {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "server")]
            #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
            $item
        )*
    }
}

macro_rules! cfg_client {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "client")]
            #[cfg_attr(docsrs, doc(cfg(feature = "client")))]
            $item
        )*
    }
}

macro_rules! cfg_server_or_client {
    ($($item:item)*) => {
        $(
            #[cfg(any(feature = "server", feature = "client"))]
            $item
        )*
    }
}

macro_rules! cfg_gateway_entity {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "gateway_entity")]
            $item
        )*
    }
}

macro_rules! cfg_factory_entity {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "factory_entity")]
            $item
        )*
    }
}

macro_rules! cfg_admin_entity {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "admin_entity")]
            $item
        )*
    }
}
