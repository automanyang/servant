// -- config.rs --

use {
    log::{info, warn},
    std::{
        fs::File,
        io::{Read, Write},
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

// --

#[cfg(feature = "adapter")]
#[cfg_attr(test, derive(Debug))]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Server {
    pub max_count_of_evictor_list: usize,
    pub serve_count_by_adapter: usize,
}
#[cfg(feature = "adapter")]
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
                    max_count_of_evictor_list: 5,
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
#[cfg(feature = "adapter")]
impl Drop for Server {
    fn drop(&mut self) {
        self.store();
    }
}

// --

#[cfg(feature = "terminal")]
#[cfg_attr(test, derive(Debug))]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Client {
    pub token_count_by_terminal: usize,
    pub callback_count_by_terminal: usize,
    pub invoke_timeout_in_terminal: u64,
}
#[cfg(feature = "terminal")]
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

#[cfg(feature = "terminal")]
impl Drop for Client {
    fn drop(&mut self) {
        self.store();
    }
}

// --

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_test_client() {
        let mut c = Client::load();
        c.invoke_timeout_in_terminal = 3000;
        dbg!(&c);
    }
    #[test]
    fn config_test_server() {
        let mut c = Server::load();
        c.max_count_of_evictor_list = 8;
        dbg!(&c);
    }
}
