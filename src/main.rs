use std::{
    fs::{self, File},
    io::{stdout, Write},
    process::{Command, ExitStatus},
    thread,
    time::Duration,
};

use bmp388::Bmp388;
use dra818v::Dra818V;
use indicatif::{style, ProgressIterator};
use neo6m::Neo6M;
use rpi_embedded::uart::{Parity, Uart};
use sc16is752::{Channel, SC16IS752};
use serde::Serialize;
use signal::SignalGenerator;

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
/// // 'O' for balloon.
/// For more info : http://www.aprs.org/symbols/symbols-new.txt
const SYMBOL: u8 = b'O';

const SC16IS752_FREQ: u32 = 1_843_200;
const SC16IS752_ID: u16 = 0x4D;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
struct Statistic {
    mean: f32,
    std_dev: f32,
}

impl Statistic {
    fn calculate(data: &[f32]) -> Statistic {
        let mean = data.iter().sum::<f32>() / data.len() as f32;

        let squared_diff: f32 = data.iter().map(|x| (x - mean) * (x - mean)).sum();
        let mean_square_diff = squared_diff / data.len() as f32;
        let std_dev = mean_square_diff.sqrt();

        Statistic { mean, std_dev }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
struct FunctionalTest {
    temperature: Statistic,
    pressure: Statistic,
    altitude: Statistic,
    latitude: Statistic,
    longitude: Statistic,
}

fn main() {
    // println!("[1/4] Capturing image...");

    // let mut cmd = Command::new("rpicam-still");
    // cmd.args(["-o", "/home/aprs/Documents/image.jpg"]);

    // let output = match cmd.output() {
    //     Ok(output) => output,
    //     Err(err) => {
    //         println!("failed to run `rpicam-still`: {err}");
    //         std::process::exit(1);
    //     }
    // };

    // if !output.status.success() {
    //     println!(
    //         "`rpicam-still` failed with message {}",
    //         String::from_utf8_lossy(&output.stderr)
    //     );
    //     std::process::exit(1);
    // }

    // let image = match fs::read("/home/aprs/Documents/image.jpg") {
    //     Ok(image) => image,
    //     Err(err) => {
    //         println!("unable to read `image.jpg`: {err}");
    //         std::process::exit(1);
    //     }
    // };

    // println!("success! image is {} bytes long", image.len());

    // println!("[2/4] Writing test APRS data...");

    // let mut generator = SignalGenerator::new().expect("unable to connect to signal generator");

    // generator.write(&[0xF0; 256]).expect("unable to write to signal generator");

    println!("[2/4] Verifying transceiver...");

    let trans_uart = Uart::new(9600, Parity::None, 8, 1).unwrap();
    let mut transceiver = Dra818V::new(trans_uart);
    println!("{:?}", transceiver.init());

    println!("[3/4] Gathering GPS readings...");

    let gps_uart = Uart::new(9600, Parity::None, 8, 1).unwrap();
    let mut gps = Neo6M::new(gps_uart);

    let mut readings = Vec::with_capacity(100);
    for _ in (0..100).progress() {
        loop {
            let available = match gps.is_available() {
                Ok(available) => available,
                Err(err) => {
                    println!("failed to check availability: {err:?}");
                    continue
                },
            };

            if available > 0 {
                match gps.read() {
                    Ok(read) => if read.longitude().is_some() && read.latitude().is_some() {
                        readings.push(read);
                        break;
                    }
                    Err(err) => {
                        println!("error reading: {err:?}");
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    let long_measurements: Vec<f32> = readings
        .iter()
        .map(|r| r.longitude().unwrap() as f32)
        .collect();
    let long_stat = Statistic::calculate(&long_measurements);

    let lat_measurements: Vec<f32> = readings
        .iter()
        .map(|r| r.latitude().unwrap() as f32)
        .collect();
    let lat_stat = Statistic::calculate(&lat_measurements);

    println!("[4/4] Capturing altimeter measurements...");

    let mut altimeter = Bmp388::new().expect("should be able to create altimeter");
    let mut measurements = Vec::with_capacity(100);

    for _ in (0..100).progress() {
        loop {
            if let Ok(measurement) = altimeter.read() {
                measurements.push(measurement);
                break;
            }
        }

        thread::sleep(Duration::from_millis(1000));
    }

    let temp_measurements: Vec<f32> = measurements.iter().map(|m| m.temperature).collect();
    let temp_stat = Statistic::calculate(&temp_measurements);

    let press_measurements: Vec<f32> = measurements.iter().map(|m| m.pressure).collect();
    let press_stat = Statistic::calculate(&press_measurements);

    let alt_measurements: Vec<f32> = measurements.iter().map(|m| m.altitude).collect();
    let alt_stat = Statistic::calculate(&alt_measurements);

    let stats = FunctionalTest {
        temperature: temp_stat,
        pressure: press_stat,
        altitude: alt_stat,
        latitude: Statistic { mean: 0.0, std_dev: 0.0 },
        longitude: Statistic { mean: 0.0, std_dev: 0.0 },
    };

    let file = File::create("ftp.json").unwrap();
    serde_json::to_writer_pretty(file, &stats).unwrap();
}
