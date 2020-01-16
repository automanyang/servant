// -- config.rs --

use {
    log::{warn, info},
    std::{
        fs::File,
        io::{Read, Write},
    },
};

// --

const CONFIG_FILE_NAME: &str = "./config.json";
lazy_static! {
    static ref CONFIGURATION: Config = Config::load(CONFIG_FILE_NAME);
}

// --

#[cfg_attr(test, derive(Debug))]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub max_count_of_evictor_list: usize,
    pub serve_count_by_adapter: usize,
    pub token_count_by_terminal: usize,
    pub callback_count_by_terminal: usize,
    pub invoke_timeout_in_terminal: u64,
}

impl Config {
    pub fn instance() -> &'static Self {
        &CONFIGURATION
    }
    fn load(file_name: &str) -> Self {
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
        match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                warn!("{}", e.to_string());
                info!("use default configuration");
                Self {
                    max_count_of_evictor_list: 5,
                    serve_count_by_adapter: 3,
                    token_count_by_terminal: 2,
                    callback_count_by_terminal: 2,
                    invoke_timeout_in_terminal: 5000
                }
            }
        }
    }
    fn store(&self, file_name: &str) {
        let json_str = match serde_json::to_string_pretty(self) {
            Ok(s) => s,
            Err(e) => {
                warn!("{}", e.to_string());
                return;
            }
        };
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

impl Drop for Config {
    fn drop(&mut self) {
        self.store(CONFIG_FILE_NAME);
    }
}

// --

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_test1() {
        let fname = "/home/xt/servant.json";
        let mut c = Config::load(fname);
        c.invoke_timeout_in_terminal = 3000;
        dbg!(&c);
        c.store(fname);
    }
}
