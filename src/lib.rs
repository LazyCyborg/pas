#![warn(clippy::all, rust_2018_idioms)]

use std::net::TcpStream;
use std::sync::{Arc, Mutex};
mod app;
pub use app::TemplateApp;
pub mod scpi_commands;
pub mod emg_scpi_commands;
pub mod siglent_data_stream;
pub mod signal;


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


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SiglentDevice {
    pub port: String,
    pub ip_address: String,

}

impl Default for SiglentDevice {
    fn default() -> Self {
        SiglentDevice {
            port: String::from("5023"),
            ip_address: String::from("169.254.165.7"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SiglentDeviceState {
    pub tcp_connection: Option<Arc<Mutex<TcpStream>>>,
}

impl Default for SiglentDeviceState {
    fn default() -> Self {
        SiglentDeviceState {
            tcp_connection: None,
        }
    }
}

pub struct WaveformPoint {
    pub time: f64,
    pub voltage: f64,
}
