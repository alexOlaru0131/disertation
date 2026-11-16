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
pub enum SensorError {
    I2c(&'static str, LinuxI2CError),
    InvalidPwmCount(usize),
}

pub struct Pca9685 {
    address: u16,
    i2c: LinuxI2CDevice,
}

impl Pca9685 {
    pub fn new(address: u16) -> Result<Self, SensorError> {
        Ok(Self {
            address,
            i2c: LinuxI2CDevice::new("/dev/i2c-1", address)
                .map_err(|e| SensorError::I2c("open PCA", e))?,
        })
    }

    fn read_reg(&self, reg: u8) -> Result<u8, SensorError> {
        self.i2c
            .smbus_read_byte_data(reg)
            .map_err(|e| SensorError::I2c("read register", e))
    }

    pub fn init(&mut self) -> Result<(), SensorError> {
        let old_mode = self.read_reg(MODE1)?;
        let old_mode2 = self.read_reg(MODE2)?;

        self.i2c
            .smbus_write_byte_data(MODE2, old_mode2 | MODE2_OUTDRV)
            .map_err(|e| SensorError::I2c("mode2 outdrv", e))?;

        let sleep_mode = (old_mode & !MODE1_RESTART) | MODE1_SLEEP;
        self.i2c
            .smbus_write_byte_data(MODE1, sleep_mode)
            .map_err(|e| SensorError::I2c("mode1 sleep", e))?;
        i2c.write(self.address, &[PRE_SCALE, PWM_FREQ_REG_VAL])
            .map_err(|e| SensorError::I2c("pre scale", e.kind()))?;

        let wake_mode = old_mode & !(MODE1_SLEEP | MODE1_RESTART);
        i2c.write(self.address, &[MODE1, wake_mode])
            .map_err(|e| SensorError::I2c("mode1 wake", e.kind()))?;

        thread::sleep(Duration::from_millis(5));
        i2c.write(self.address, &[MODE1, wake_mode | MODE1_RESTART | MODE1_AI])
            .map_err(|e| SensorError::I2c("mode1 restart", e.kind()))?;
        Ok(())
    }

    pub fn set_pwm_vals<I: I2c>(
        &mut self,
        i2c: &mut I,
        pwm_centidegrees: &[u16],
    ) -> Result<(), SensorError> {
        if pwm_centidegrees.len() < self.number_of_servos as usize {
            return Err(SensorError::InvalidPwmCount(pwm_centidegrees.len()));
        }

        for (channel, &centidegrees) in pwm_centidegrees
            .iter()
            .take(self.number_of_servos as usize)
            .enumerate()
        {
            let ticks = angle_to_ticks(centidegrees);
            let bytes = [
                LED0_ON_L + 4 * channel as u8,
                0,
                0,
                ticks as u8,
                ((ticks >> 8) & 0x0F) as u8,
            ];
            i2c.write(self.address, &bytes)
                .map_err(|e| SensorError::I2c("set pwm", e.kind()))?;
        }
        Ok(())
    }
}

fn angle_to_ticks(centidegrees: u16) -> u16 {
    let centidegrees = centidegrees.min(36_000) as f32;
    let pulse_us = MIN_US + (centidegrees / 36_000.0) * (MAX_US - MIN_US);
    (pulse_us * 50.0 * 4096.0 / 1_000_000.0 + 0.5) as u16
}

pub(crate) fn update_pwm<I: I2c, D: bsp::ehal::delay::DelayNs>(
    pwm: &mut Pca9685,
    initialized: &mut bool,
    i2c: &mut I,
    delay: &mut D,
    angles: &[u16],
) -> Result<(), SensorError> {
    if !*initialized {
        pwm.init(i2c, delay)?;
        *initialized = true;
    }

    if let Err(error) = pwm.set_pwm_vals(i2c, angles) {
        *initialized = false;
        return Err(error);
    }
    Ok(())
}
