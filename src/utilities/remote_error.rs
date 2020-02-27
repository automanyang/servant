// -- spot_error.rs

use serde::{Serialize, Deserialize};

// --

#[macro_export]
macro_rules! on_the_remote {
    () => {
        RemoteError::new(file!().to_string(), line()!, String::new());
    };
    ($val:ident) => {
        RemoteError::new(file!().to_string(), line!(), $val.to_string());
    };
    ($val:expr) => {
        RemoteError::new(file!().to_string(), line!(), $val);
    };
}

#[macro_export]
macro_rules! show {
    () => {
        println!("[{}:{}]", file!(), line!());
    };
    ($val:expr) => {
        match $val {
            tmp => {
                println!("[{}:{}] {} = {}",
                    file!(), line!(), stringify!($val), tmp);
            }
        }
    };
    ($val:expr,) => { show!($val) };
    ($($val:expr),+ $(,)?) => {
        ($(show!($val)),+,)
    };
}

#[macro_export]
macro_rules! output {
    () => {
        println!("[{}:{}]", file!(), line!());
    };
    ($val:expr) => {
        match $val {
            tmp => {
                println!("[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), tmp);
            }
        }
    };
    ($val:expr,) => { output!($val) };
    ($($val:expr),+ $(,)?) => {
        ($(output!($val)),+,)
    };
}

// --

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteError {
    file: String,
    line: u32,
    desc: String,
}
impl RemoteError {
    pub fn new(file: String, line: u32, desc: String) -> Self {
        Self {
            file,
            line,
            desc,
        }
    }
}

impl std::error::Error for RemoteError {}

impl std::fmt::Display for RemoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RemoteError({}: {}, {})",
            self.file, self.line, self.desc
        )
    }
}

pub type RemoteResult<T> = Result<T, RemoteError>;
pub type GeneralResult<T> = Result<T, Box<dyn std::error::Error>>;
pub type GeneralResultWithSend<T> = Result<T, Box<dyn std::error::Error + Send>>;
// pub type GeneralResultWithSendSync<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
