// -- admin.rs --
use crate::{
    self as servant,
    adapter::AdapterRegister,
    servant::{Context, ServantRegister, UserCookie, Oid},
    task,
    utilities::{RemoteError, RemoteResult},
};
use log::info;
use rand::random;
use std::{net::SocketAddr};

// --

#[servant::invoke_interface]
pub trait Admin {
    fn acquire(&self, password: String) -> RemoteResult<UserCookie>;
    fn shutdown(&self, passcode: usize) -> RemoteResult<()>;
    fn adapter_list(&self) -> RemoteResult<Vec<SocketAddr>>;
    fn servants(&self) -> RemoteResult<Vec<Oid>>;
    fn report_servants(&self) -> RemoteResult<Vec<Oid>>;
    fn watch_servant(&self) -> RemoteResult<bool>;
    // fn evictor_list(&self) -> RemoteResult<Vec<Oid>>;
}

// --

pub struct AdminEntity {
    password: String,
    shutdown_code: usize,
    cookie: UserCookie,
    ar: AdapterRegister,
    sr: ServantRegister,
}
impl AdminEntity {
    pub fn new(
        password: &str,
        shutdown_code: usize,
        ar: AdapterRegister,
        sr: ServantRegister,
    ) -> Self {
        Self {
            password: password.to_owned(),
            shutdown_code,
            cookie: 0,
            ar,
            sr,
        }
    }
    fn check_user_cookie(&self, ctx: Option<Context>) -> bool {
        ctx.and_then(|v| v.user_cookie)
            .and_then(|v| if v == self.cookie { Some(true) } else { None })
            .unwrap_or(false)
    }
    fn update_cookie(&mut self) {
        self.cookie = random();
    }
}
impl Admin for AdminEntity {
    fn acquire(&self, _ctx: Option<Context>, password: String) -> RemoteResult<UserCookie> {
        if !password.is_empty() && password == self.password {
            unsafe {
                let s = self as *const Self as *mut Self;
                &mut *s
            }
            .update_cookie();
            Ok(self.cookie)
        } else {
            Err(on_the_remote!("password invalid".to_owned()))
        }
    }
    fn shutdown(&self, ctx: Option<Context>, passcode: usize) -> RemoteResult<()> {
        if self.check_user_cookie(ctx) && passcode != 0 && passcode == self.shutdown_code {
            let ar = self.ar.clone();
            task::block_on(async {
                ar.clean().await;
            });
            info!("shutdown by admin");
            Ok(())
        } else {
            info!("shutdown with invalid context.");
            Err(on_the_remote!("invalid context".to_owned()))
        }
    }
    fn adapter_list(&self, ctx: Option<Context>) -> RemoteResult<Vec<SocketAddr>> {
        if !self.check_user_cookie(ctx) {
            return Err(on_the_remote!("invalid context".to_owned()));
        }
        task::block_on(async { Ok(self.ar.list().await) })
    }
    fn servants(&self, ctx: Option<Context>) -> RemoteResult<Vec<Oid>> {
        if !self.check_user_cookie(ctx) {
            return Err(on_the_remote!("invalid context".to_owned()));
        }
        task::block_on(async { 
            Ok(self.sr.servants().await)
        })
    }
    fn report_servants(&self, ctx: Option<Context>) -> RemoteResult<Vec<Oid>> {
        if !self.check_user_cookie(ctx) {
            return Err(on_the_remote!("invalid context".to_owned()));
        }
        task::block_on(async { 
            Ok(self.sr.report_servants().await)
        })
    }
    fn watch_servant(&self, ctx: Option<Context>) -> RemoteResult<bool> {
        if !self.check_user_cookie(ctx) {
            return Err(on_the_remote!("invalid context".to_owned()));
        }
        task::block_on(async { 
            Ok(self.sr.watch_servant().await.is_some())
        })
    }
}
