// -- config.rs --

cfg_server_or_client! {
use {
    log::{info, warn},
    std::{
        fs::File,
        io::{Read, Write},
        collections::HashMap,
    },
};

// --

fn read_json_str(file_name: &str) -> String {
    let mut json_str = String::new();
    let json_str = match File::open(file_name) {
        Ok(mut file) => {
            if let Err(e) = file.read_to_string(&mut json_str) {
                warn!("{}", e.to_string());
            }
            json_str
        }
        Err(e) => {
            warn!("{}", e.to_string());
            json_str
        }
    };
    json_str
}
fn store_json_str(file_name: &str, json_str: &str) {
    match File::create(file_name) {
        Ok(mut f) => {
            if let Err(e) = f.write(json_str.as_bytes()) {
                warn!("{}", e.to_string());
            }
        }
        Err(e) => {
            warn!("{}", e.to_string());
        }
    };
}
}

// --

cfg_server! {
// #[cfg_attr(test, derive(Debug))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct HelpData {
    pub(crate) name: String,
    pub(crate) about: String,
    pub(crate) readme: String,
    pub(crate) version: String,
    pub(crate) context: HashMap<String, String>,
}
impl HelpData {
    fn new() -> Self {
        Self {
            name: String::new(),
            about: String::new(),
            readme: String::new(),
            version: String::new(),
            context: HashMap::new(),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct AdminData {
    pub(crate) name: String,
    pub(crate) password: String,
    pub(crate) shutdown_code: usize,
}
impl AdminData {
    fn new() -> Self {
        Self {
        name: String::new(),
        password: String::new(),
        shutdown_code: 0,
    }
}
}

// #[cfg_attr(test, derive(Debug))]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Server {
    pub(crate) help: HelpData,
    pub(crate) admin: AdminData,
    pub max_count_of_evictor_list: usize,
    pub max_count_of_connection: usize,
    pub serve_count_by_adapter: usize,
}
impl Server {
    fn file_name() -> &'static str {
        "./server.json"
    }
    pub fn load() -> Self {
        let json_str = read_json_str(Self::file_name());
        match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                warn!("{}", e.to_string());
                info!("server use default configuration");
                Self {
                    help: HelpData::new(),
                    admin: AdminData::new(),
                    max_count_of_evictor_list: 5,
                    max_count_of_connection: 10,
                    serve_count_by_adapter: 3,
                }
            }
        }
    }
    pub fn store(&self) {
        let json_str = match serde_json::to_string_pretty(self) {
            Ok(s) => s,
            Err(e) => {
                warn!("{}", e.to_string());
                return;
            }
        };
        store_json_str(Self::file_name(), &json_str);
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.store();
    }
}
}

// --

cfg_client! {
#[cfg_attr(test, derive(Debug))]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Client {
    pub admin_cookie: String,
    pub token_count_by_terminal: usize,
    pub callback_count_by_terminal: usize,
    pub invoke_timeout_in_terminal: u64,
}
impl Client {
    fn file_name() -> &'static str {
        "./client.json"
    }
    pub fn load() -> Self {
        let json_str = read_json_str(Self::file_name());
        match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                warn!("{}", e.to_string());
                info!("client use default configuration");
                Self {
                    admin_cookie: String::new(),
                    token_count_by_terminal: 2,
                    callback_count_by_terminal: 2,
                    invoke_timeout_in_terminal: 5000,
                }
            }
        }
    }
    pub fn store(&self) {
        let json_str = match serde_json::to_string_pretty(self) {
            Ok(s) => s,
            Err(e) => {
                warn!("{}", e.to_string());
                return;
            }
        };
        store_json_str(Self::file_name(), &json_str);
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.store();
    }
}
}

// --

cfg_server_or_client! {
mod tests {

    #[test]
    fn config_test_client() {
        use super::*;
        let mut c = Client::load();
        c.invoke_timeout_in_terminal = 3000;
        dbg!(&c);
    }
    #[test]
    fn config_test_server() {
        use super::*;
        let mut c = Server::load();
        c.max_count_of_evictor_list = 8;
        dbg!(&c);
    }
}
}
