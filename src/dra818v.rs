use rpi_embedded::uart::{self, Uart};

pub struct Dra818V {
    uart: Uart,
}

impl Dra818V {
    pub fn new(uart: Uart) -> Self {
        return Self { uart }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.handshake()?;
        self.set_group(144.390, 144.390)?;

        Ok(())
    }

    pub fn set_group(&mut self, tfv: f32, rfv: f32) -> Result<(), Error> {
        self.uart.write(format!("AT+DMOSETGROUP=0,{:.4},{:.4},0000,0,0000\r\n", tfv, rfv))?;
        println!("AT+DMOSETGROUP=0,{:.4},{:.4},0000,4,0000\r\n", tfv, rfv);
        let line = self.uart.read_line()?;

        if line != "+DMOSETGROUP:0\r" {
            return Err(Error::NoConnect);
        }

        Ok(())
    }

    pub fn handshake(&mut self) -> Result<(), Error> {
        self.uart.write_bytes(b"AT+DMOCONNECT\r\n")?;
        let line = self.uart.read_line()?;

        if line != "+DMOCONNECT:0\r" {
            return Err(Error::NoConnect);
        }

        Ok(())
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
