// -- admin.rs --

use {
    crate::{
        self as servant,
        adapter::AdapterRegister,
        servant::{Context},
    },
    async_std::task,
};

// --

#[servant::invoke_interface]
pub trait Admin {
    fn shutdown(&self, passcode: usize);
}

// --

pub struct AdminEntity {
    ar: AdapterRegister,
}
impl AdminEntity {
    pub fn new(ar: AdapterRegister) -> Self {
        Self { ar }
    }
}
impl Admin for AdminEntity {
    fn shutdown(&self, _ctx: Option<Context>, passcode: usize) {
        let ar = self.ar.clone();
        task::block_on(async {
            ar.clean(passcode).await;
        });
    }
}
