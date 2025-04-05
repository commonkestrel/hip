use nmea::{Nmea, SentenceType};
use rpi_embedded::uart::{Queue, Uart};
use thiserror::Error;

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
        let mut sentence = String::new();
        loop {
            sentence = self.uart.read_line()?;
            if sentence.starts_with('$') {
                break;
            }
        }

        let mut nmea = Nmea::create_for_navigation(&[SentenceType::TXT, SentenceType::GGA])?;
        nmea.parse(&sentence)?;

        return Ok(nmea);
    }

    pub fn flush(&mut self) -> Result<(), GpsError> {
        self.uart.flush(Queue::Input)?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum GpsError {
    #[error("failed to recieve data from the serial bus: {0}")]
    Uart(#[from] rpi_embedded::uart::Error),
    #[error("no data available from the serial bus")]
    DataUnavailable,
    #[error("failed to parse NMEA sentence: {0}")]
    Nmea(String),
}

impl<'a> From<nmea::Error<'a>> for GpsError {
    fn from(value: nmea::Error<'a>) -> Self {
        GpsError::Nmea(value.to_string())
    }
}
