use i2cdev::{
    core::I2CDevice,
    linux::{LinuxI2CDevice, LinuxI2CError},
};
use std::{fmt, thread, time::Duration};

const MODE1: u8 = 0x00;
const MODE2: u8 = 0x01;
const LED0_ON_L: u8 = 0x06;
const PRE_SCALE: u8 = 0xFE;
const PWM_FREQ_REG_VAL: u8 = 0x79;
const MODE1_RESTART: u8 = 0x80;
const MODE1_AI: u8 = 0x20;
const MODE1_SLEEP: u8 = 0x10;
const MODE2_OUTDRV: u8 = 0x04;

const MIN_US: f32 = 500.0;
const MAX_US: f32 = 2400.0;

#[derive(Debug)]
pub enum Error {
    I2c(&'static str, LinuxI2CError),
    InvalidPwmCount(usize),
    InvalidAddress(u16),
    InvalidAngle(usize, u16),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

pub struct Pca9685 {
    address: u16,
    i2c: LinuxI2CDevice,
}

impl Pca9685 {
    pub fn new(address: u16) -> Result<Self, Error> {
        Ok(Self {
            address,
            i2c: LinuxI2CDevice::new("/dev/i2c-1", address)
                .map_err(|e| Error::I2c("open PCA", e))?,
        })
    }

    fn read_reg(&mut self, reg: u8) -> Result<u8, Error> {
        self.i2c
            .smbus_read_byte_data(reg)
            .map_err(|e| Error::I2c("read register", e))
    }

    pub fn init(&mut self) -> Result<(), Error> {
        let old_mode = self.read_reg(MODE1)?;
        let old_mode2 = self.read_reg(MODE2)?;

        self.i2c
            .smbus_write_byte_data(MODE2, old_mode2 | MODE2_OUTDRV)
            .map_err(|e| Error::I2c("mode2 outdrv", e))?;

        let sleep_mode = (old_mode & !MODE1_RESTART) | MODE1_SLEEP;
        self.i2c
            .smbus_write_byte_data(MODE1, sleep_mode)
            .map_err(|e| Error::I2c("mode1 sleep", e))?;
        self.i2c
            .smbus_write_byte_data(PRE_SCALE, PWM_FREQ_REG_VAL)
            .map_err(|e| Error::I2c("pre scale", e))?;

        let wake_mode = old_mode & !(MODE1_SLEEP | MODE1_RESTART);
        self.i2c
            .smbus_write_byte_data(MODE1, wake_mode)
            .map_err(|e| Error::I2c("mode1 wake", e))?;

        thread::sleep(Duration::from_millis(5));
        self.i2c
            .smbus_write_byte_data(MODE1, wake_mode | MODE1_RESTART | MODE1_AI)
            .map_err(|e| Error::I2c("mode1 restart", e))?;
        Ok(())
    }

    fn angle_to_ticks(centidegrees: u16) -> u16 {
        let centidegrees = centidegrees as f32;
        let pulse_us = MIN_US + (centidegrees / 36_000.0) * (MAX_US - MIN_US);
        (pulse_us * 50.0 * 4096.0 / 1_000_000.0 + 0.5) as u16
    }

    pub fn set_pwm_vals(&mut self, pwm_vals: &[u16]) -> Result<(), Error> {
        if pwm_vals.len() != 10 {
            return Err(Error::InvalidPwmCount(pwm_vals.len()));
        }
        let start_channel = match self.address {
            0x42 => 0,
            0x41 => 6,
            address => return Err(Error::InvalidAddress(address)),
        };
        if let Some(index) = pwm_vals.iter().position(|&angle| angle > 36_000) {
            return Err(Error::InvalidAngle(index, pwm_vals[index]));
        }

        for (channel, &centidegrees) in pwm_vals.iter().enumerate() {
            let ticks = Self::angle_to_ticks(centidegrees);
            let bytes = [
                LED0_ON_L + 4 * (start_channel + channel) as u8,
                0,
                0,
                ticks as u8,
                ((ticks >> 8) & 0x0F) as u8,
            ];
            self.i2c
                .write(&bytes)
                .map_err(|e| Error::I2c("set pwm", e))?;
        }
        Ok(())
    }
}
