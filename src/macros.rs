// -- macro.rs --

#![allow(unused_macros)]

// --

macro_rules! cfg_adapter {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "adapter")]
            #[cfg_attr(docsrs, doc(cfg(feature = "adapter")))]
            $item
        )*
    }
}

macro_rules! cfg_terminal {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "terminal")]
            #[cfg_attr(docsrs, doc(cfg(feature = "terminal")))]
            $item
        )*
    }
}