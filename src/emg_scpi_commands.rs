use ::std::io;
use std::io::{BufReader, BufRead};
use ::std::io::Write;
use std::net::{TcpStream, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{SiglentDevice, SiglentDeviceState, DeviceState, WaveformPoint, siglent_data_stream};

// TIMEOUT FOR NETWORK OPERATIONS
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(200);
// DEFAULT TIME OUT AFTER OCCILSCOPE RESET
const DEFAULT_TIME_AFTER_RESET: Duration = Duration::from_millis(2000);
// MINIMUM TIME ALLOWED BETWEEN PULSES
const DEFAULT_TIME_BETWEEN_PULSES: Duration = Duration::from_millis(1000);

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


// TCP CONNECTION
pub fn connect_to_occiloscope(siglent_device: &mut SiglentDevice, siglent_device_state: &mut SiglentDeviceState) -> io::Result<()> {
    if siglent_device_state.tcp_connection.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Already connected to a device",
        ));
    }
    if siglent_device.ip_address.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "IP address not specified",
        ));
    }
    if siglent_device.port.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Port not specified",
        ));
    }
    let ip_address_and_port = format!("{}:{}", siglent_device.ip_address, siglent_device.port);
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
    siglent_device_state.tcp_connection = Some(stream);

    Ok(())
}

// SETUP FOR SIGLENT OCCILOSCOPE

pub fn reset_siglent_device(siglent_device: &mut SiglentDevice, siglent_device_state: &mut SiglentDeviceState) -> io::Result<()> {
    let mut stream = match siglent_device_state.tcp_connection.as_mut() {
        Some(stream) => stream,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Not connected to device",
            ));
        }
    };

    write_to_stream(&mut stream, "*RST", true)?;
    //let _ = get_response_from_stream(&mut stream);
    std::thread::sleep(DEFAULT_TIME_AFTER_RESET);
    connect_to_occiloscope(siglent_device, siglent_device_state)?;
    println!("Reconnected to device after reset");

    Ok(())
}


pub fn configure_siglent_device(
    siglent_device_state: &mut SiglentDeviceState,
) -> io::Result<()> {
    let mut stream = match siglent_device_state.tcp_connection.as_mut() {
        Some(stream) => stream,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Not connected to device",
            ));
        }
    };

    write_to_stream(&mut stream, "*RST", true)?;
    std::thread::sleep(DEFAULT_TIME_AFTER_RESET);
    // Turn channel 1 on which is going to record EMG
    write_to_stream(&mut stream, "C1:TRA ON", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    // Turn channel 2 on
    write_to_stream(&mut stream, "C2:TRA ON", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    // Set trigger mode to normal
    //write_to_stream(&mut stream, "TRMD NORM", true)?;
    //std::thread::sleep(DEFAULT_TIMEOUT);
    // Set trigger slope to rising (default) and trigger type to edge
    write_to_stream(&mut stream, "TRSE EDGE,SR,C2", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    // Set trigger level to 2V
    write_to_stream(&mut stream, "C2:TRLV 2V", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "MSIZ 70M", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    // Collect 100 ms of data
    write_to_stream(&mut stream, "SARA 20M", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "C1:VDIV 100MV", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "TDIV 10MS", true)?;
    write_to_stream(&mut stream, "ACQW?", false)?;
    let acq_mode = siglent_data_stream::read_ascii_response(&mut stream)?;
    println!("Acquisition mode: {}", acq_mode);
    std::thread::sleep(DEFAULT_TIMEOUT);
    // Collect all data in memory buffer
    write_to_stream(&mut stream, "WaveForm_SetUp TYPE 1", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "TRMD NORMAL", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);

    write_to_stream(&mut stream, "MSIZ?", false)?;
    let memory_size = siglent_data_stream::read_ascii_response(&mut stream)?;
    println!("Memory depth set to: {}", memory_size);


    //C1:FILTS TYPE,BR,LOWLIMIT,45Hz,UPPLIMIT,55Hz



    Ok(())
}


// KEYSIGHT FOR EMG
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

    write_to_stream(&mut stream, "SOURce1:FUNCtion PULSe", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce1:FREQuency 100", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce1:BURSt:STATe ON", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce1:BURSt:MODE TRIGgered", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce1:BURSt:NCYCles 1", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);


    write_to_stream(&mut stream, "SOURce2:FUNCtion PULSe", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce2:FREQuency 100", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce2:BURSt:STATe ON", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce2:BURSt:MODE TRIGgered", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);
    write_to_stream(&mut stream, "SOURce2:BURSt:NCYCles 1", true)?;
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

pub fn turn_on_ch2(device_state: &mut DeviceState) -> io::Result<()> {
    let mut stream = match device_state.tcp_connection.as_mut() {
        Some(stream) => stream,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Not connected to device",
            ));
        }
    };
    write_to_stream(&mut stream, "OUTPut2:STATe ON", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);

    Ok(())
}
pub fn stimulate_and_send_trigger(
    stream: &Arc<Mutex<TcpStream>>,
) -> io::Result<()> {

        write_to_stream(stream, "*TRG", true)?;
        println!("STIMULATING!");
        std::thread::sleep(DEFAULT_TIME_BETWEEN_PULSES);


    Ok(())
}


pub fn stimulate_send_trigger_record_emg(
    keysight_stream: &Arc<Mutex<TcpStream>>,
    siglent_stream: &Arc<Mutex<TcpStream>>,
) -> io::Result<(Vec<WaveformPoint>, f64)>{

    write_to_stream(siglent_stream, "TRMD SINGLE", true)?;
    std::thread::sleep(DEFAULT_TIMEOUT);

    println!("**** Writing to Keysight stream ****");
    write_to_stream(keysight_stream, "*TRG", true)?;
    println!("**** STIMULATING! **** \n");


    write_to_stream(siglent_stream, "WAIT", true)?;
    write_to_stream(siglent_stream, "TRMD?", false)?;
    let trig_status = siglent_data_stream::read_ascii_response(siglent_stream)?;
    println!("Trigger status: {}", trig_status);
    write_to_stream(siglent_stream, "C1:VDIV?", false)?;
    let v_div = siglent_data_stream::parse_scpi_value(&siglent_data_stream::read_ascii_response(siglent_stream)?).unwrap_or(1.0);
    println!("Vertical division: {}", v_div);
    write_to_stream(siglent_stream, "C1:OFST?", false)?;
    let offset = siglent_data_stream::parse_scpi_value(&siglent_data_stream::read_ascii_response(siglent_stream)?).unwrap_or(0.0);
    println!("Offset: {}", offset);
    write_to_stream(siglent_stream, "TRDL?", false)?;
    let trigger_delay = siglent_data_stream::parse_scpi_value(&siglent_data_stream::read_ascii_response(siglent_stream)?).unwrap_or(0.0);
    println!("Trigger delay: {}", trigger_delay);
    let grid_divisions = 10.0;
    let code_per_div = 25.0;
    let center_code = 128.0;
    write_to_stream(siglent_stream, "C1:WF? DAT2", true)?;
    let raw_adc_values = siglent_data_stream::read_waveform_block(siglent_stream)?;
    let total_points = raw_adc_values.len();
    if total_points == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No waveform data received from oscilloscope",
        ));
    }
    let time_div: f64 = 20.0e-3;
    let time_window = time_div * grid_divisions;
    let actual_sample_rate = total_points as f64 / time_window;
    println!("Calculated actual sample rate: {} Sa/s", actual_sample_rate);
    println!("Received {} data points.", total_points);
    let time_start = trigger_delay - (time_window / 2.0);
    let waveform: Vec<WaveformPoint> = raw_adc_values
        .into_iter()
        .enumerate()
        .map(|(index, adc_value)| {
            let voltage = (adc_value as f64 - center_code) * (v_div / code_per_div) - offset;
            let time = time_start + (index as f64) * (1.0 / actual_sample_rate);
            WaveformPoint { time, voltage }
        })
        .collect();
    Ok((waveform, actual_sample_rate))
}


// //             //write_to_stream(siglent_stream, "TRMD NORM", true)?;
// //             write_to_stream(keysight_stream, "*TRG", true)?;
// println!("STIMULATING!");
// std::thread::sleep(DEFAULT_TIME_BETWEEN_AQUISITIONS);
// //
// //
