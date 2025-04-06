use std::{thread, time::Duration};

use log::warn;
use rpi_embedded::uart::{self, Uart};

pub struct Dra818V {
    uart: Uart,
}

impl Dra818V {
    pub fn new(uart: Uart) -> Self {
        return Self { uart }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.uart.set_read_mode(0, Duration::ZERO)?;
        self.uart.set_write_mode(true)?;
        self.handshake()?;
        self.set_group(144.390, 144.390)?;

        Ok(())
    }

    pub fn set_group(&mut self, tfv: f32, rfv: f32) -> Result<(), Error> {
        self.uart.write(format!("AT+DMOSETGROUP=0,{:.4},{:.4},0000,0,0000\r\n", tfv, rfv))?;
        println!("AT+DMOSETGROUP=0,{:.4},{:.4},0000,4,0000\r\n", tfv, rfv);
        self.terrible_timeout()?;
        let mut line = self.uart.read_line()?;
        if line.is_empty() {
            line = self.uart.read_line()?;
        }

        if line != "+DMOSETGROUP:0\r" {
            return Err(Error::NoConnect);
        }

        Ok(())
    }

    pub fn handshake(&mut self) -> Result<(), Error> {
        self.uart.write_bytes(b"AT+DMOCONNECT\r\n")?;
        self.terrible_timeout()?;
        let mut line = self.uart.read_line()?;
        if line.is_empty() {
            line = self.uart.read_line()?;
        }

        if line != "+DMOCONNECT:0\r" {
            warn!("transceiver confirmation not recieved: {line}");
            return Err(Error::NoConnect);
        }

        Ok(())
    }

    fn terrible_timeout(&self) -> Result<(), Error> {
        if self.uart.input_len()? == 0 {
            thread::sleep(Duration::from_millis(750));
            if self.uart.input_len()? == 0 {
                return Err(Error::NoConnect);
            }
        }

        return Ok(());
    }
}

#[derive(Debug)]
pub enum Error {
    Uart(uart::Error),
    NoConnect,
}

impl From<uart::Error> for Error {
    fn from(value: uart::Error) -> Self {
        return Self::Uart(value);
    }
}
