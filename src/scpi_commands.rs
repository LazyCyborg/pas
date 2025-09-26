
use ::std::io;
use ::std::io::{BufRead, BufReader, Write};
use std::sync::mpsc::Sender;
use std::net::{TcpStream, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;


use crate::{KeysightDevice, DeviceState};

// TIMEOUT FOR NETWORK OPERATIONS
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(200);

// DELAY BETWEEN PULSES IN MS
const DEFAULT_TIME_BETWEEN_PULSES: Duration = Duration::from_millis(5000);

// NUMBER OF PULSES IN STIMULATION PROTOCOL
const DEFAULT_NUMBER_OF_PULSES: i32 = 240;

//UTILITY FUNCTIONS FOR SCPI COMMUNICATION
fn write_to_stream(stream: &Arc<Mutex<TcpStream>>, command: &str, verify_send: bool) -> io::Result<()> {
    let mut stream = stream.lock().unwrap();
    let command = format!("{}\r\n", command);
    if verify_send {
        println!("Sending command to device: {}", command.trim());
    }

    stream.write_all(command.as_bytes())?;
    stream.flush()?;
    Ok(())
}


// fn get_response_from_stream(stream: &Arc<Mutex<TcpStream>>) -> io::Result<()> {
//     let mut stream = stream.lock().unwrap();
//     let mut byte_buf = Vec::new();
//     let mut buffer = [0; 1024]; // Temporary buffer to read data
//     let mut reader = BufReader::new(&stream);
//     reader.read_until(b'\n', &mut byte_buf)?;
//     let response = String::from_utf8_lossy(&byte_buf);

//     if byte_buf.len() > 0 {
//         println!("{}", response.trim());
//
//     Ok(())
// }


// TCP CONNECTION
pub fn connect_to_keysight(keysight_device: &mut KeysightDevice, device_state: &mut DeviceState) -> io::Result<()> {
    if device_state.tcp_connection.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Already connected to a device",
        ));
    }
    if keysight_device.ip_address.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "IP address not specified",
        ));
    }

    if keysight_device.port.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Port not specified",
        ));
    }
    let ip_address_and_port = format!("{}:{}", keysight_device.ip_address, keysight_device.port);

    let socket_addr = match ip_address_and_port.parse::<SocketAddr>() {
        Ok(addr) => addr,
        Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid IP address or port format")),
    };

    let timeout = Duration::from_secs(5);

    let stream = TcpStream::connect_timeout(&socket_addr, timeout).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to connect to {}: {}", ip_address_and_port, e),
        )
    })?;

    std::thread::sleep(DEFAULT_TIMEOUT);

    let mut reader = BufReader::new(&stream);
    let mut byte_buf = Vec::new();
    reader.read_until(b'\n', &mut byte_buf)?;
    let response = String::from_utf8_lossy(&byte_buf);

    if byte_buf.len() > 0 {
        println!("{}", response.trim());
    }
    let stream = Arc::new(Mutex::new(stream));
    device_state.tcp_connection = Some(stream);

    Ok(())
}

pub fn reset_device(keysight_device: &mut KeysightDevice, device_state: &mut DeviceState) -> io::Result<()> {
    let mut stream = match device_state.tcp_connection.as_mut() {
        Some(stream) => stream,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Not connected to device",
            ));
        }
    };

    write_to_stream(&mut stream, "*RST", true)?;
    std::thread::sleep(Duration::from_secs(5));
    connect_to_keysight(keysight_device, device_state)?;
    println!("Reconnected to device after reset");

    Ok(())
}



pub fn set_impedance(device_state: &mut DeviceState) -> io::Result<()> {
    let mut stream = match device_state.tcp_connection.as_mut() {
        Some(stream) => stream,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Not connected to device",
            ));
        }
    };

    write_to_stream(&mut stream, "OUTPut1:LOAD INFinity", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);

    write_to_stream(&mut stream, "OUTPut2:LOAD INFinity", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);

    // write_to_stream(&mut stream, "OUTPut1:LOAD?", true)?;
    // std::thread::sleep(DEFAULT_TIMEOUT);
    // _ = get_response_from_stream(&mut stream);

    // write_to_stream(&mut stream, "OUTPut2:LOAD?", true)?;
    // std::thread::sleep(DEFAULT_TIMEOUT);
    // _ = get_response_from_stream(&mut stream);

    Ok(())
}

pub fn configure_channel_1(
    device_state: &mut DeviceState
) -> io::Result<()> {
    let mut stream = match device_state.tcp_connection.as_mut() {
        Some(stream) => stream,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Not connected to device",
            ));
        }
    };

    write_to_stream(&mut stream, "SOURce1:FUNCtion PULSe", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce1:BURSt:STATe ON", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce1:BURSt:MODE TRIGgered", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce1:BURSt:NCYCles 1", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce1:VOLTage 5", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
//


    Ok(())
}

pub fn configure_channel_2(
    device_state: &mut DeviceState,
    amplitude_channel_2: f64,
    pulse_width_channel_2: f64,
) -> io::Result<()> {
    let mut stream = match device_state.tcp_connection.as_mut() {
        Some(stream) => stream,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Not connected to device",
            ));
        }
    };

    match amplitude_channel_2 {
        0.0..=20.0 => {
            write_to_stream(
                &mut stream,
                &format!("SOURce2:VOLTage:LOW {}", 0.0),
                true,
            )?;
            write_to_stream(
                &mut stream,
                &format!("SOURce2:VOLTage:HIGH {}", amplitude_channel_2),
                true,
            )?;
            std::thread::sleep(DEFAULT_TIMEOUT);
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Amplitude out of safe range",
            ));
        }
    }

    write_to_stream(&mut stream, "SOURce2:FUNCtion PULSe", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce2:FREQuency 100", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce2:BURSt:STATe ON", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce2:BURSt:MODE TRIGgered", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce2:BURSt:NCYCles 6", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);


    match pulse_width_channel_2 {
        0.0001..=0.0008 => {
            write_to_stream(
                &mut stream,
                &format!("SOURce2:FUNCtion:PULSe:WIDTh {}", pulse_width_channel_2),
                true,
            )?;
            std::thread::sleep(DEFAULT_TIMEOUT);
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Pulse width out of safe range",
            ));
        }
    }


    Ok(())
}


pub fn arm_for_trigger(device_state: &mut DeviceState) -> io::Result<()> {
    let mut stream = match device_state.tcp_connection.as_mut() {
        Some(stream) => stream,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Not connected to device",
            ));
        }
    };
    write_to_stream(&mut stream, "TRIGger1:SOURce BUS", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "TRIGger2:SOURce BUS", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "INITiate:CONTinuous:ALL ON", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);


    Ok(())
}

pub fn turn_on_ch1_ch2(device_state: &mut DeviceState) -> io::Result<()> {
    let mut stream = match device_state.tcp_connection.as_mut() {
        Some(stream) => stream,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Not connected to device",
            ));
        }
    };
    write_to_stream(&mut stream, "OUTPut1:STATe ON", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "OUTPut2:STATe ON", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);

    Ok(())
}

pub fn stimulate_and_send_trigger(
    stream: &Arc<Mutex<TcpStream>>,
    progress_sender: Sender<String>,
    is_stimulating: Arc<Mutex<bool>>,
) -> io::Result<()> {
    println!("NUMBER OF STEPS {:?} \n", DEFAULT_NUMBER_OF_PULSES);
    let number_of_pulse_vector: Vec<i32> = (1..=DEFAULT_NUMBER_OF_PULSES).collect();
    print!("RANGE {:?} \n", number_of_pulse_vector.len());
    let mut current_pulse_index = 0;



    let number_of_pulse_vector: Vec<i32> = (1..=DEFAULT_NUMBER_OF_PULSES).collect();
        for _ in number_of_pulse_vector.iter() {
            if !*is_stimulating.lock().unwrap() {
                break;
            }
            write_to_stream(stream, "*TRG", true)?;
            current_pulse_index += 1;
            let message = format!("Stimulating: Pulse {} / {}", current_pulse_index, DEFAULT_NUMBER_OF_PULSES);
            if progress_sender.send(message).is_err() {
                println!("GUI closed. Stopping stimulation thread.");
                break;
            }
            println!("Pulse index/total: {} / {}", current_pulse_index, DEFAULT_NUMBER_OF_PULSES);
            std::thread::sleep(DEFAULT_TIME_BETWEEN_PULSES);
        }


    Ok(())
}


pub fn show_query_commands() -> io::Result<()> {
    writeln!(
        std::io::stdout(),
        "Available SCPI query commands for the Keysight EDU33210 Series Trueform Arbitrary Waveform Generator:"
    )?;
    writeln!(
        std::io::stdout(),
        "SOURce1:FREQuency? - Returns the output frequency for Channel 1."
    )?;
    writeln!(
        std::io::stdout(),
        "SOURce2:FREQuency? - Returns the output frequency for Channel 2."
    )?;
    writeln!(
        std::io::stdout(),
        "SOURce1:VOLTage? - Returns the output amplitude for Channel 1."
    )?;
    writeln!(
        std::io::stdout(),
        "SOURce2:VOLTage? - Returns the output amplitude for Channel 2."
    )?;
    writeln!(
        std::io::stdout(),
        "OUTPut1[:STATe]? - Returns the enabled/disabled state of the front panel output connector for Channel 1. It will return 0 for OFF or 1 for ON."
    )?;
    writeln!(
        std::io::stdout(),
        "OUTPut2[:STATe]? - Returns the enabled/disabled state of the front panel output connector for Channel 2. It will return 0 for OFF or 1 for ON."
    )?;
    writeln!(
        std::io::stdout(),
        "See https://www.keysight.com/us/en/assets/9921-01382/programming-guides/EDU33210-Series-Trueform-Arbitrary-Waveform-Generator-Programming-Guide.pdf for full list"
    )?;

    Ok(())
}

//See https://www.keysight.com/us/en/assets/9921-01382/programming-guides/EDU33210-Series-Trueform-Arbitrary-Waveform-Generator-Programming-Guide.pdf for full list
