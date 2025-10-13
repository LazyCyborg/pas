
use ::std::io::{Read, BufReader, BufRead};
use std::io::{self};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Mutex;



pub fn read_ascii_response(stream: &Arc<Mutex<TcpStream>>) -> io::Result<String> {
    let stream_lock = stream.lock().unwrap();
    let mut reader = BufReader::new(&*stream_lock);
    let mut response_buf = Vec::new();
    reader.read_until(b'\n', &mut response_buf)?;
    Ok(String::from_utf8_lossy(&response_buf).trim().to_string())
}

pub fn parse_scpi_value(response: &str) -> Result<f64, &'static str> {
    if let Some(value_str) = response.split_whitespace().last() {
        value_str.parse::<f64>().map_err(|_| "Failed to parse value")
    } else {
        Err("Invalid SCPI response format")
    }
}

pub fn read_waveform_block(stream: &Arc<Mutex<TcpStream>>) -> io::Result<Vec<u8>> {
    let stream_lock = stream.lock().unwrap();
    let mut reader = BufReader::new(&*stream_lock);

    reader.read_until(b'#', &mut Vec::new())?;
    let mut len_digit_buf = [0u8; 1];
    reader.read_exact(&mut len_digit_buf)?;
    let len_digits = (len_digit_buf[0] as char).to_digit(10).unwrap_or(0) as usize;

    let mut len_buf = vec![0u8; len_digits];
    reader.read_exact(&mut len_buf)?;
    let data_length = std::str::from_utf8(&len_buf).unwrap_or("0").parse::<usize>().unwrap_or(0);

    let mut waveform_data = vec![0u8; data_length];
    reader.read_exact(&mut waveform_data)?;

    let mut terminator_buf = [0u8; 2];
    reader.read_exact(&mut terminator_buf)?;

    Ok(waveform_data)
}
