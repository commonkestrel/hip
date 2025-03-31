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
const FCR: u8 = 0x02;
const MCR: u8 = 0x04;
const DLL: u8 = 0x00;
const DLH: u8 = 0x01;
const SPR: u8 = 0x07;

pub struct SC16IS752 {
    i2c: I2c,
}

impl SC16IS752 {
    pub fn begin(
        addr: u16,
        baud_a: u32,
        baud_b: u32,
        crystal_freq: u32,
        data_length: DataLength,
        parity: Parity,
        stop_length: StopLength,
    ) -> i2c::Result<Self> {
        let mut i2c = I2c::with_bus(1)?;
        // Use 0x4D when both A0 and A1 are connected to ground
        i2c.set_slave_address(addr)?;

        let mut this = Self { i2c };
        this.ping();
        this.reset()?;
        this.fifo_enable(Channel::A)?;
        this.fifo_enable(Channel::B)?;
        this.set_baudrate(Channel::A, baud_a, crystal_freq)?;
        this.set_baudrate(Channel::B, baud_b, crystal_freq)?;
        this.set_line(Channel::A, data_length, parity, stop_length)?;
        this.set_line(Channel::B, data_length, parity, stop_length)?;

        Ok(this)
    }

    pub fn reset(&mut self) -> i2c::Result<()> {
        let mut reg = self.read_reg(Channel::Both, IOCONTROL)?;
        reg |= 0x08;
        self.write_reg(Channel::Both, IOCONTROL, reg)?;

        Ok(())
    }

    fn fifo_enable(&mut self, channel: Channel) -> i2c::Result<()> {
        let mut fcr = self.read_reg(channel, FCR)?;
        fcr |= 0x01;
        self.write_reg(channel, FCR, fcr)?;

        Ok(())
    }

    pub fn set_baudrate(&mut self, channel: Channel, baud: u32, crystal_freq: u32) -> i2c::Result<()> {
        let prescaler = if self.read_reg(channel, MCR)? & 0x80 == 0 {
            1
        } else {
            4
        };
        let divisor1 = crystal_freq / prescaler;
        let divisor2 = baud * 16;

        if divisor2 > divisor1 {
            return Err(i2c::Error::Io(io::Error::new(
                ErrorKind::InvalidInput,
                "the specified baud rate is not valid",
            )));
        }

        let wk = (divisor1 as f64) / (divisor2 as f64);
        let divisor = wk.ceil() as u16;

        println!("divisor: {divisor}");

        let mut lcr = self.read_reg(channel, LCR)?;
        lcr |= 0x80;
        self.write_reg(channel, LCR, lcr)?;

        self.write_reg(channel, DLL, (divisor & 0xFF) as u8)?;
        self.write_reg(channel, DLH, (divisor << 8) as u8)?;

        lcr &= 0x7F;
        self.write_reg(channel, LCR, lcr)?;

        Ok(())
    }

    fn set_line(
        &mut self,
        channel: Channel,
        data_length: DataLength,
        parity: Parity,
        stop_length: StopLength,
    ) -> i2c::Result<()> {
        let mut lcr = self.read_reg(channel, LCR)?;
        lcr &= 0xC0;

        match data_length {
            DataLength::D5 => {},
            DataLength::D6 => lcr |= 0x01,
            DataLength::D7 => lcr |= 0x02,
            DataLength::D8 => lcr |= 0x03,
        }

        match stop_length {
            StopLength::One => lcr &= !0x04,
            StopLength::Two => lcr |= 0x04,
        }

        match parity {
            Parity::None => {},
            Parity::Odd => lcr |= 0x08,
            Parity::Even => lcr |= 0x18,
            Parity::One => lcr |= 0x03,
            Parity::Zero => {},
        }

        self.write_reg(channel, LCR, lcr)?;

        Ok(())
    }

    pub fn write_byte(&mut self, channel: Channel, byte: u8) -> i2c::Result<()> {
        while self.read_reg(channel, LCR)? & 0x20 == 0 {}
        self.write_reg(channel, THR_RHR, byte)?;

        Ok(())
    }

    pub fn write(&mut self, channel: Channel, buf: &[u8]) -> i2c::Result<()> {
        for byte in buf {
            self.write_byte(channel, *byte)?;
        }

        Ok(())
    }

    pub fn read_byte(&self, channel: Channel) -> i2c::Result<u8> {
        if self.available(channel)? == 0 {
            return Err(i2c::Error::Io(io::Error::new(
                ErrorKind::WouldBlock,
                "no data ready in the FIFO buffer",
            )));
        }
        let byte = self.read_reg(channel, THR_RHR)?;

        Ok(byte)
    }

    /// Reads bytes until a newline is reached.
    /// This method will block waiting for new input,
    pub fn read_line(&self, channel: Channel) -> i2c::Result<String> {
        let mut bytes = Vec::new();

        loop {
            if self.available(channel)? > 0 {
                let byte = self.read_byte(channel)?;
                if byte == b'\n' {
                    break;
                }

                bytes.push(self.read_byte(channel)?);
            } else {
                // Let the processor know that we are in a spin loop,
                // but probably not for very long.
                hint::spin_loop();
            }
        }

        let string = match String::from_utf8(bytes) {
            Ok(string) => string,
            Err(err) => {
                return Err(i2c::Error::Io(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid UTF-8 in data",
                )))
            }
        };

        return Ok(string);
    }

    pub fn read_with_timeout(&self, channel: Channel, timeout: Duration) -> i2c::Result<u8> {
        let start = SystemTime::now();

        while self.available(channel)? > 0 {
            if start.elapsed().unwrap() > timeout {
                return Err(i2c::Error::Io(io::Error::new(
                    ErrorKind::TimedOut,
                    "timed out waiting for available bytes",
                )));
            }
        }

        self.read_reg(channel, THR_RHR)
    }

    pub fn available(&self, channel: Channel) -> i2c::Result<usize> {
        self.read_reg(channel, RX_LVL)
            .map(|available| available as usize)
    }

    fn read_reg(&self, channel: Channel, reg: u8) -> i2c::Result<u8> {
        self.i2c.smbus_read_byte(reg << 3 | channel.select())
    }

    fn write_reg(&mut self, channel: Channel, reg: u8, value: u8) -> i2c::Result<()> {
        self.i2c.write(&[reg << 3 | channel.select(), value])?;

        Ok(())
    }

    pub fn ping(&mut self) -> i2c::Result<()> {
        self.write_reg(Channel::A, SPR, 0x55)?;

        if self.read_reg(Channel::A, SPR)? != 0x55 {
            panic!("failed on A SPR");
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataLength {
    D5,
    D6,
    D7,
    D8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Parity {
    None,
    Odd,
    Even,
    One,
    Zero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StopLength {
    One,
    Two,
}
