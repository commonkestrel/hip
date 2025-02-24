use std::{mem::{self, MaybeUninit}, thread, time::Duration};

use rpi_embedded::i2c::{self, I2c};

const CHIP_ID_REGISTER: u8 = 0x00;
const ERROR_REGISTER: u8 = 0x02;
const STATUS_REGISTER: u8 = 0x03;
const PRESSURE_REGISTER: u8 = 0x04;
const TEMPERATURE_REGISTER: u8 = 0x07;
const POWER_CTRL_REGISTER: u8 = 0x1B;
const BMP388_ADDRESS: u16 = 0x77;
const COMMAND_REGISTER: u8 = 0x7E;

const SOFT_RESET_COMMAND: u8 = 0xB6;

const CMD_READY_MASK: u8 = 0x10;
const COMMAND_ERROR_MASK: u8 = 0x02;

const SEA_LEVEL_PRESSURE: f32 = 102201.21;

pub struct Bmp388 {
    i2c: I2c
}

impl Bmp388 {
    pub fn new() -> Result<Self, AltimeterError> {
        let mut i2c = I2c::with_bus(1)?;
        i2c.set_slave_address(BMP388_ADDRESS)?;

        let this = Self {i2c};
        this.init()?;

        Ok(this)
    }

    pub fn read(&mut self) -> rpi_embedded::i2c::Result<AltimeterData> {
        let mut data = [0; 6];

        self.i2c.write_read(&[PRESSURE_REGISTER], &mut data)?;

        // Yeah this is stupid but it works :3
        let mut coef_uninit: MaybeUninit<IntCoefficients> = MaybeUninit::uninit();
        let coef_slice: &mut [u8; 21] = unsafe { mem::transmute(coef_uninit.as_mut_ptr()) };
        self.i2c.write_read(&[0x31], coef_slice)?;
        let coef = unsafe { coef_uninit.assume_init() };

        let raw_temperature = u32::from_be_bytes([0x00, data[5], data[4], data[3]]);
        let temperature = self.compensate_temperature(raw_temperature as f32, coef)?;
        
        let raw_pressure = u32::from_be_bytes([0x00, data[2], data[1], data[0]]);
        let pressure = self.compensate_pressure(raw_pressure as f32, temperature, coef)?;

        let altitude = ((SEA_LEVEL_PRESSURE / pressure).powf(0.190223) - 1.0) * (temperature + 273.15) / 0.0065;

        Ok(AltimeterData {pressure, temperature, altitude})
    }

    fn init(&self) -> Result<(), AltimeterError> {
        self.reset()?;

        let chip_id = self.i2c.smbus_read_byte(CHIP_ID_REGISTER)?;
        self.i2c.smbus_write_byte(POWER_CTRL_REGISTER, 0b00110011)?;
        self.i2c.smbus_write_byte(0x1C, 0)?;
        self.i2c.smbus_write_byte(0x1D, 0)?;

        Ok(())
    }

    pub fn reset(&self) -> Result<(), AltimeterError> {
        let status = self.i2c.smbus_read_byte(STATUS_REGISTER)?;
        if status & CMD_READY_MASK == 0 {
            return Err(AltimeterError::CommandFailed);
        }
        
        self.i2c.smbus_write_byte(COMMAND_REGISTER, SOFT_RESET_COMMAND)?;
        thread::sleep(Duration::from_micros(2000));
        let result = self.i2c.smbus_read_byte(ERROR_REGISTER)?;
        if result & COMMAND_ERROR_MASK > 0 {
            return Err(AltimeterError::CommandFailed);
        }

        return Ok(());
    }

    fn compensate_temperature(&self, raw_temp: f32, coef: IntCoefficients) -> i2c::Result<f32> {
        let nvm_t1 = coef.t1 as f32;
        let t1 = nvm_t1 / 2.0f32.powi(-8);

        let nvm_t2 = coef.t2 as f32;
        let t2 = nvm_t2 / 2.0f32.powi(30);

        let nvm_t3 = coef.t3 as f32;
        let t3 = nvm_t3 / 2.0f32.powi(48);

        println!("nvm_t1: {nvm_t1}, nvm_t2: {nvm_t2}, nvm_t3: {nvm_t3}");

        let partial1 = raw_temp as f32 - t1;
        let partial2 = partial1 * t2 as f32;

        return Ok(partial2 + (partial1 * partial1) * t3)
    }

    fn compensate_pressure(&self, raw_press: f32, temp: f32, coef: IntCoefficients) -> i2c::Result<f32> {
        // Absolute garbage from the BMP388 datasheet (pg. 56) 
        let p1 = ((coef.p1 as f32) - 2.0f32.powi(14)) / 2.0f32.powi(20);
        let p2 = ((coef.p2 as f32) - 2.0f32.powi(14)) / 2.0f32.powi(29);
        let p3 = (coef.p3 as f32) / 2.0f32.powi(32);
        let p4 = (coef.p4 as f32) / 2.0f32.powi(37);
        let p5 = (coef.p5 as f32) / 2.0f32.powi(-3);
        let p6 = (coef.p6 as f32) / 2.0f32.powi(6);
        let p7 = (coef.p7 as f32) / 2.0f32.powi(8);
        let p8 = (coef.p8 as f32) / 2.0f32.powi(15);
        let p9 = (coef.p9 as f32) / 2.0f32.powi(48);
        let p10 = (coef.p10 as f32) / 2.0f32.powi(48);
        let p11 = (coef.p11 as f32) / 2.0f32.powi(65);

        let partial1 = p6 * temp;
        let partial2 = p7 * (temp * temp);
        let partial3 = p8 * (temp * temp * temp);
        let partial_out1 = p5 + partial1 + partial2 + partial3;

        let partial4 = p2 * temp;
        let partial5 = p3 * temp;
        let partial6 = p4 * temp;
        let partial_out2 = raw_press * (p1 + partial4 + partial5 + partial6);

        let partial7 = raw_press * raw_press;
        let partial8 = p9 + p10 * temp;
        let partial9 = partial7 * partial8;
        let partial_out3 = partial9 + (raw_press * raw_press * raw_press) * p11;

        let comp_press = partial_out1 + partial_out2 + partial_out3;

        return Ok(comp_press);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AltimeterData {
    // Pressure in Pascals
    pressure: f32,
    // Temperature in Celcius
    temperature: f32,
    // Altitude in meters
    altitude: f32,
}

#[derive(Debug)]
pub enum AltimeterError {
    I2C(rpi_embedded::i2c::Error),
    CommandFailed,
}

impl From<rpi_embedded::i2c::Error> for AltimeterError {
    fn from(value: rpi_embedded::i2c::Error) -> Self {
        AltimeterError::I2C(value)
    }
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
struct IntCoefficients {
    t1: u16, 
    t2: u16,
    t3: i8,
    p1: i16,
    p2: i16,
    p3: i8,
    p4: i8,
    p5: u16,
    p6: u16,
    p7: i8,
    p8: i8,
    p9: i16,
    p10: i8,
    p11: i8,
}
