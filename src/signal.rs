//! Communicating with the custom ATTiny85 program over I2C to generate the APRS audio signal.

use rpi_embedded::i2c::I2c;

pub struct SignalGenerator {
    i2c: I2c,
}

impl SignalGenerator {}
