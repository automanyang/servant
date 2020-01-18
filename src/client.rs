// -- client.rs --

use {
    crate::{config, servant::ServantResult, terminal::Terminal},
};

// --

pub struct Client {
    config: config::Client,
}
impl Client {
    pub fn new() -> Self {
        Self {
            config: config::Client::load(),
        }
    }
    pub async fn connect_to(&self, addr: String) -> ServantResult<Terminal> {
        let t = Terminal::new(
            addr,
            self.config.invoke_timeout_in_terminal,
            self.config.token_count_by_terminal,
            self.config.callback_count_by_terminal,
        );
        if let Err(e) = t.connect_to().await {
            Err(e.to_string().into())
        } else {
            Ok(t)
        }
    }
}
