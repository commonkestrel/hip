use std::{
    fs::{self, File},
    io::Write,
    process::{Command, ExitStatus},
    thread,
    time::Duration,
};

use neo6m::Neo6M;
use rpi_embedded::uart::{Parity, Uart};

mod aprs;
mod bmp388;
mod neo6m;

// TODO: DO NOT FORGET TO CHANGE
const CALLSIGN: &[u8; 6] = b"NOCALL";
/// [Balloon SSID](http://www.aprs.org/aprs11/SSIDs.txt)
const SSID: u8 = 11;
/// // 'O' for balloon.
/// For more info : http://www.aprs.org/symbols/symbols-new.txt
const SYMBOL: u8 = b'O';

fn main() {
    let mut cmd = Command::new("rpicam-still");
    cmd.args(["-o", "/home/aprs/Documents/image.jpg"]);

    let output = match cmd.output() {
        Ok(output) => output,
        Err(err) => {
            println!("failed to run `rpicam-still`: {err}");
            std::process::exit(1);
        }
    };

    if !output.status.success() {
        println!(
            "`rpicam-still` failed with message {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::process::exit(1);
    }

    let image = match fs::read("/home/aprs/Documents/image.jpg") {
        Ok(image) => image,
        Err(err) => {
            println!("unable to read `image.jpg`: {err}");
            std::process::exit(1);
        }
    };

    println!("success! image is {} bytes long", image.len());

    let gps_uart = Uart::new(9600, Parity::Even, 8, 1).expect("should be able to create uart");
    let mut gps = Neo6M::new(gps_uart);

    for i in 0..5 {
        while !gps.is_available().unwrap() {}
        println!("{}", gps.read().unwrap());
    }
}
