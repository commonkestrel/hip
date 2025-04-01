use nmea::{Nmea, SentenceType};
use rpi_embedded::uart::Uart;

use crate::sc16is752::{Channel, SC16IS752};

pub struct Neo6M {
    uart: Uart,
}

impl Neo6M {
    pub fn new(uart: Uart) -> Self {
        return Self { uart };
    }

    pub fn is_available(&self) -> Result<usize, GpsError> {
        Ok(self.uart.input_len()?)
    }

    pub fn read(&mut self) -> Result<Nmea, GpsError> {
        // Just checking `is_available` without fighting the borrow checker
        if self.uart.input_len()? == 0 {
            return Err(GpsError::DataUnavailable);
        }

        let sentence = self.uart.read_line()?;
        // println!("{sentence}");

        let mut nmea = Nmea::create_for_navigation(&[SentenceType::TXT, SentenceType::GGA])?;
        nmea.parse(&sentence)?;

        return Ok(nmea);
    }
}

#[derive(Debug)]
pub enum GpsError {
    Uart(rpi_embedded::uart::Error),
    DataUnavailable,
    Nmea,
}

impl From<rpi_embedded::uart::Error> for GpsError {
    fn from(value: rpi_embedded::uart::Error) -> Self {
        GpsError::Uart(value)
    }
}

impl<'a> From<nmea::Error<'a>> for GpsError {
    fn from(value: nmea::Error<'a>) -> Self {
        GpsError::Nmea
    }
}
