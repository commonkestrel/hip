use std::{
    fs::{self, File}, io::{self, stdout, Write}, iter, process::{Command, ExitStatus}, thread, time::Duration
};

use bmp388::Bmp388;
use dra818v::Dra818V;
use ftail::Ftail;
use log::{info, warn};
use neo6m::Neo6M;
use num_bigint::BigUint;
use rpi_embedded::{gpio::{Gpio, Level}, i2c, uart::{Parity, Uart}};
use sc16is752::{Channel, SC16IS752};
use signal::SignalGenerator;
use thiserror::Error;
use ssdv::encoder::{Encoder, EncodeError};
use chrono::Timelike;

mod aprs;
mod ax25;
mod bmp388;
mod neo6m;
mod sc16is752;
mod signal;
mod dra818v;

// TODO: DO NOT FORGET TO CHANGE
const CALLSIGN: &[u8; 6] = b"NOCALL";
/// [Balloon SSID](http://www.aprs.org/aprs11/SSIDs.txt)
const SSID: u8 = 11;
/// Destination callsign
const DEST_CALLSIGN: &[u8; 6] = b"APRS  ";
const DEST_SSID: u8 = 0;
/// // 'O' for balloon.
/// For more info : http://www.aprs.org/symbols/symbols-new.txt
const SYMBOL: u8 = b'O';

const SC16IS752_FREQ: u32 = 1_843_200;
const SC16IS752_ID: u16 = 0x4D;

const MAX_RETRIES: usize = 20;

const METERS_TO_FEET: f32 = 3.280839895;

const GPS_LEVEL: Level = Level::Low;
const TRANSCEIVER_LEVEL: Level = Level::High; 

const FLAG_SIZE: usize = 20;

fn main() -> ! {
    if let Err(err) = Ftail::new().console(log::LevelFilter::Debug).single_file("/home/aprs/Documents/log.txt", true, log::LevelFilter::Debug).init() {
        println!("Error initializing ftail logging: {err}");
    }

    let gpio = Gpio::new().expect("Should be able to capture GPIO");
    let mut uart_select = gpio.get(26).expect("Should be able to capture UART select pin").into_output();

    uart_select.write(TRANSCEIVER_LEVEL);
    let trans_uart = Uart::new(9600, Parity::None, 8, 1).unwrap();
    let mut transceiver = Dra818V::new(trans_uart);
    
    // Retry initialization of tranceiver until success
    while let Err(err) = transceiver.init() {
        warn!("Failed to initialize tranceiver (retrying in 1s): {err:?}");
        thread::sleep(Duration::from_millis(1000));
    }

    // Retry initialization of signal generator until success
    let mut generator;
    loop {
        match SignalGenerator::new() {
            Ok(gen) => {
                generator = gen;
                break;
            }
            Err(err) => {
                warn!("Failed to initialize signal generator (retrying in 1s): {err}");
                thread::sleep(Duration::from_millis(1000));
            }
        }
    }

    // yeah i broke the altimeter so this is commented out until i fix it

    // Retry initialization of altimeter until success
    let mut altimeter;
    loop {
        match Bmp388::new() {
            Ok(alt) => {
                altimeter = alt;
                break;
            }
            Err(err) => {
                warn!("Failed to initialize altimeter (retrying in 1s): {err:?}");
                thread::sleep(Duration::from_millis(1000));
            }
        }
    }

    uart_select.write(GPS_LEVEL);
    let gps_uart = Uart::new(9600, Parity::None, 8, 1).unwrap();
    let mut gps = Neo6M::new(gps_uart);

    let mut transmitting_image = false;
    let mut packet_num = 0;
    let mut image_packet_num = 0;
    let mut image_packet_data: Option<[u8; 256]> = None;
    let mut ssdv_iter: Box<dyn Iterator<Item = Result<[u8; 256], EncodeError>>> = Box::new(iter::empty());
    loop {
        let mut retries = 0;

        if transmitting_image && image_packet_num != 4 {
            if image_packet_num % 2 == 0 {
                loop {
                    match ssdv_iter.next() {
                        Some(Ok(data)) => {
                            info!("Successfully generated SSDV packet");
                            image_packet_data = Some(data);
                            break;
                        }
                        Some(Err(err)) => {
                            warn!("Failed to generate SSDV packet: {err:?}");
                        }
                        None => {
                            info!("Reached the end of the image");
                            image_packet_data = None;
                            break;
                        }
                    }
                }
            }

            if let Some(ref data) = image_packet_data {
                let mut image_retries = 0;
                while image_retries < MAX_RETRIES {
                    match transmit_image_packet(packet_num, data, image_packet_num % 2 == 0, &mut generator) {
                        Ok(_) => break,
                        Err(err) => {
                            warn!("Failed to transmit image packet: {err}");
                            image_retries += 1;
                        }
                    }
                }
            }
            
            image_packet_num += 1;
        } else {
            image_packet_num = 0;
            while retries < MAX_RETRIES {
                match transmit_location(packet_num, &mut gps, &mut altimeter, &mut generator) {
                    Ok(_) => break,
                    Err(err) => {
                        warn!("failed to transmit location: {err}");
                        retries += 1;
                        thread::sleep(Duration::from_millis(1000));
                    },
                }
            }
        }

        if !transmitting_image {
            match gps.read().map(|reading| reading.altitude()) {
                Ok(alt) => if let Some(20_000.0..) = alt {
                    let mut image_retries = 0;
                    while image_retries < MAX_RETRIES {
                        match capture_image() {
                            Ok(image) => {
                                ssdv_iter = Box::new(Encoder::new(*CALLSIGN, 1, ssdv::Quality::Q1, image));
                                transmitting_image = true;
                                break;
                            },
                            Err(err) => {
                                warn!("failed to capture image: {err}");
                                image_retries += 1;
                            }
                        }
                    }
                }
                Err(err) => warn!("failed to check altimeter for image capture: {err}"),
            }
        }

        packet_num += 1;
        thread::sleep(Duration::from_secs(60));
    }
}

fn transmit_location(packet_num: usize, gps: &mut Neo6M, altimeter: &mut Bmp388, generator: &mut SignalGenerator) -> Result<(), Error> {
    let location = gps.read()?;
    let altimeter_data = altimeter.read().map_err(|err| Error::Altimeter(err))?;

    let (longitude, latitude, time) = match (location.longitude(), location.latitude(), location.fix_timestamp()) {
        (Some(long), Some(lat), Some(time)) => (long, lat, time),
        _ => return Err(Error::GpsData),
    };

    let mut data = Vec::new();
    write_header(&mut data, packet_num);
    data.push(b'/');
    data.extend(format!("{:02}{:02}{:02}h", time.hour(), time.minute(), time.second()).bytes());

    let (lat_deg, lat_min, lat_sign) = if latitude < 0.0 {
        let deg = latitude.abs().floor();
        (deg as usize, (latitude.abs() - deg) * 60.0, 'S')
    } else {
        let deg = latitude.floor();
        (deg as usize, (latitude - deg) * 60.0, 'N')
    };

    let (long_deg, long_min, long_sign) = if longitude < 0.0 {
        let deg = longitude.abs().floor();
        (deg as usize, (longitude.abs() - deg) * 60.0, 'W')
    } else {
        let deg = longitude.floor();
        (deg as usize, (longitude - deg) * 60.0, 'E')
    };

    data.extend(format!("{:0>2}{:0>5.2}{}/{:0>3}{:0>2.2}{}", lat_deg, lat_min, lat_sign, long_deg, long_min, long_sign).bytes());

    // Balloon symbol code
    data.push(b'O');
    data.push(b' ');

    if let (Some(speed), Some(course)) = (location.speed_over_ground, location.true_course) {
        data.extend(format!("{:0>3}/{:0>3}", course.ceil() as isize, speed.ceil() as isize).bytes());
    }

    data.extend(format!("/A={:0>6}", (altimeter_data.altitude * METERS_TO_FEET).round() as usize).bytes());
    data.extend(format!("/Pa={:0>6}", altimeter_data.pressure.round() as usize).bytes());
    data.extend(format!("/Ti={:.2}", altimeter_data.temperature).bytes());
    
    let mut crc: u16 = 0xffff;

    for bit in data.iter().skip(FLAG_SIZE).map(|byte| (0..8).map(move |i| (byte >> i) & 0x01)).flatten() {
        crc ^= bit as u16;

        if crc & 0x01 > 0 {
            crc = (crc >> 1) ^ 0x8408;
        } else {
            crc >>= 1;
        }
    }

    data.push((crc ^ 0xFF) as u8);
    data.push(((crc >> 8) ^ 0xFF) as u8);

    data.extend_from_slice(&[0x7e; FLAG_SIZE]);

    info!("Sending APRS location packet: \"{}\"", String::from_utf8_lossy(&data));
    fs::write("/home/aprs/Documents/packet.bin", &data[FLAG_SIZE..data.len()-FLAG_SIZE]).unwrap();

    generator.write(&data).map_err(|err| Error::Generator(err))?;

    Ok(())
}

fn transmit_image_packet(packet_num: usize, packet_data: &[u8], second: bool, generator: &mut SignalGenerator) -> Result<(), Error> {
    let mut data = Vec::new();

    write_header(&mut data, packet_num);

    data.extend_from_slice(b"{{I");

    data.extend_from_slice(&packet_data[0..ssdv::encoder::HEADER_SIZE]);
    if second {
        // yes this is bs shush
        data.append(&mut b91_encode(&packet_data[ssdv::encoder::HEADER_SIZE+ssdv::encoder::PAYLOAD_SIZE/2..packet_data.len()-ssdv::encoder::CRC_SIZE]));
    } else {
        data.append(&mut b91_encode(&packet_data[ssdv::encoder::HEADER_SIZE.. ssdv::encoder::HEADER_SIZE+ssdv::encoder::PAYLOAD_SIZE/2]));
    }

    let mut crc: u16 = 0xffff;

    for bit in data.iter().skip(FLAG_SIZE).map(|byte| (0..8).map(move |i| (byte >> i) & 0x01)).flatten() {
        crc ^= bit as u16;

        if crc & 0x01 > 0 {
            crc = (crc >> 1) ^ 0x8408;
        } else {
            crc >>= 1;
        }
    }

    data.push((crc ^ 0xFF) as u8);
    data.push(((crc >> 8) ^ 0xFF) as u8);

    data.extend_from_slice(&[0x7e; FLAG_SIZE]);

    generator.write(&data).map_err(|err| Error::Generator(err))?;
    
    Ok(())
}

fn write_header(buf: &mut Vec<u8>, packet_num: usize) {
    buf.extend_from_slice(&[0x7e; FLAG_SIZE]);
    buf.extend(DEST_CALLSIGN.iter().map(|byte| byte << 1));
    buf.push((DEST_SSID + b'0') << 1);
    buf.extend(CALLSIGN.iter().map(|byte| byte << 1));
    buf.push ((SSID + b'0') << 1 | 1);
    buf.push(0x03);
    buf.push(0xf0);
}

fn capture_image() -> Result<Vec<u8>, io::Error> {
    let mut cmd = Command::new("rpicam-still");
    cmd.args(["-o", "/home/aprs/Documents/image.jpg", "-q", "50"]);

    let output = cmd.output()?;

    if !output.status.success() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, String::from_utf8_lossy(&output.stderr)));
    }

    return fs::read("/home/aprs/Documents/image.jpg");
}

fn b91_encode(buf: &[u8]) -> Vec<u8> {
    let mut num = BigUint::from_bytes_le(buf);
    let mut output = Vec::new();

    while num >= BigUint::from(91u8) {
        let remainder: u8 = (&num % 91u8).try_into().unwrap();
        output.push(remainder + 33);

        num /= 91u8;
    }

    let remainder: u8 = num.try_into().unwrap();
    output.push(remainder + 33);
    output.reverse();

    return output;
}

#[derive(Debug, Error)]
enum Error {
    #[error("Failed to read GPS data: {0}")]
    Gps(#[from] neo6m::GpsError),
    #[error("GPS data contains no location")]
    GpsData,
    #[error("Failed to read altimeter data: {0}")]
    Altimeter(i2c::Error),
    #[error("Failed to transmit to signal generator: {0}")]
    Generator(i2c::Error),
}
