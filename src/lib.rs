#![warn(clippy::all, rust_2018_idioms)]

use std::net::TcpStream;
use std::sync::{Arc, Mutex};
mod app;
pub use app::TemplateApp;
pub mod scpi_commands;


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeysightDevice {
    pub port: String,
    pub ip_address: String,

}


impl Default for KeysightDevice {
    fn default() -> Self {
        KeysightDevice {
            port: String::from("5024"),
            ip_address: String::from("169.254.165.7"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceState {
    pub tcp_connection: Option<Arc<Mutex<TcpStream>>>,
}

impl Default for DeviceState {
    fn default() -> Self {
        DeviceState {
            tcp_connection: None,
        }
    }
}
