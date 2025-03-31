//! Communicating with the custom ATTiny85 program over I2C to generate the APRS audio signal.

use rpi_embedded::i2c::{self, I2c};

const GENERATOR_ADDR: u16 = 0x40;

pub struct SignalGenerator {
    i2c: I2c,
}

impl SignalGenerator {
    pub fn new() -> i2c::Result<Self> {
        let mut i2c = I2c::new()?;
        i2c.set_slave_address(GENERATOR_ADDR)?;

        Ok(Self {i2c})
    }

    pub fn write(&mut self, buf: &[u8]) -> i2c::Result<()> {
        for chunk in buf.chunks(15) {
            self.i2c.write(chunk)?;
        }

        Ok(())
    }
}
