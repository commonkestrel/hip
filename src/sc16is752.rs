use std::{
    hint,
    io::{self, ErrorKind, Read, Write},
    time::{Duration, SystemTime},
};

use rpi_embedded::i2c::{self, I2c};

const LCR: u8 = 0x05;
const THR_RHR: u8 = 0x00;
const RX_LVL: u8 = 0x09;
const IOCONTROL: u8 = 0x0E;

pub struct SC16IS752 {
    i2c: I2c,
}

impl SC16IS752 {
    pub fn begin(addr: u16, baud: u16, crystal_freq: u32) -> i2c::Result<Self> {
        let i2c = I2c::new()?;
        // Use 0x4D when both A0 and A1 are connected to ground
        i2c.set_slave_address(addr);

        let this = Self { i2c };
        this.reset();

    }

    pub fn reset(&self) {

    }

    pub fn write_byte(&self, channel: Channel, byte: u8) -> i2c::Result<()> {
        Ok(())
    }

    pub fn read_byte(&self, channel: Channel) -> i2c::Result<u8> {
        while self.avaliable(channel)? == 0 {
            hint::spin_loop();
        }

        self.read_reg(channel, THR_RHR)
    }

    pub fn read_with_timeout(&self, channel: Channel, timeout: Duration) -> i2c::Result<u8> {
        let start = SystemTime::now();

        while self.avaliable(channel) {
            if start.elapsed().unwrap() > timeout {
                return Err(i2c::Error::Io(io::Error::new(
                    ErrorKind::TimedOut,
                    "timed out waiting for available bytes",
                )));
            }

            self.read_reg(channel, THR_RHR)
        }
    }

    pub fn avaliable(&self, channel: Channel) -> i2c::Result<usize> {
        self.read_reg(channel, RX_LVL)
            .map(|available| available as usize)
    }

    fn read_reg(&self, channel: Channel, reg: u8) -> i2c::Result<u8> {
        self.i2c.smbus_read_byte(reg << 3 | channel.select())
    }

    fn write_reg(&self, channel: Channel, reg: u8, value: u8) -> i2c::Result<()> {
        self.i2c
            .smbus_write_byte(reg << 3 | channel.select(), value)
    }
}

impl Write for SC16IS752 {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for byte in buf {
            self.write_byte(byte, Channel::Both)?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Read for SC16IS752 {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        todo!()
    }
}

pub enum ControlFlow {
    None,
    Xon1Xoff1 {
        xon1: u8,
        xoff1: u8,
    },
    Xon2Xoff2 {
        xon2: u8,
        xoff2: u8,
    },
    Xon12Xoff12 {
        xon1: u8,
        xon2: u8,
        xoff1: u8,
        xoff2: u8,
    },
}

pub enum Channel {
    A,
    B,
    Both,
}

impl Channel {
    fn select(&self) -> u8 {
        match self {
            Channel::A => 0x00,
            Channel::B => 0x02,
            Channel::Both => 0x00,
        }
    }
}
