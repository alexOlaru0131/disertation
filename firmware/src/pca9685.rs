use feather_m0::ehal::delay::DelayNs;
use feather_m0::ehal::i2c::{Error as _, ErrorKind, I2c};

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
    I2c(&'static str, ErrorKind),
    InvalidServoCount(u8),
    InvalidPwmCount(usize),
}

pub struct Pca9685 {
    number_of_servos: u8,
    address: u8,
}

impl Pca9685 {
    pub fn new(number_of_servos: u8, address: u8) -> Self {
        Self {
            number_of_servos,
            address,
        }
    }

    fn read_reg<I: I2c>(&self, i2c: &mut I, reg: u8) -> Result<u8, SensorError> {
        let mut data = [0];
        i2c.write_read(self.address, &[reg], &mut data)
            .map_err(|e| SensorError::I2c("read register", e.kind()))?;
        Ok(data[0])
    }

    pub fn init<I: I2c, D: DelayNs>(
        &mut self,
        i2c: &mut I,
        delay: &mut D,
    ) -> Result<(), SensorError> {
        let old_mode = self.read_reg(i2c, MODE1)?;
        let old_mode2 = self.read_reg(i2c, MODE2)?;

        i2c.write(self.address, &[MODE2, old_mode2 | MODE2_OUTDRV])
            .map_err(|e| SensorError::I2c("mode2 outdrv", e.kind()))?;

        let sleep_mode = (old_mode & !MODE1_RESTART) | MODE1_SLEEP;
        i2c.write(self.address, &[MODE1, sleep_mode])
            .map_err(|e| SensorError::I2c("mode1 sleep", e.kind()))?;
        i2c.write(self.address, &[PRE_SCALE, PWM_FREQ_REG_VAL])
            .map_err(|e| SensorError::I2c("pre scale", e.kind()))?;

        let wake_mode = old_mode & !(MODE1_SLEEP | MODE1_RESTART);
        i2c.write(self.address, &[MODE1, wake_mode])
            .map_err(|e| SensorError::I2c("mode1 wake", e.kind()))?;

        delay.delay_ms(5);
        i2c.write(self.address, &[MODE1, wake_mode | MODE1_RESTART | MODE1_AI])
            .map_err(|e| SensorError::I2c("mode1 restart", e.kind()))?;
        Ok(())
    }

    pub fn set_pwm_vals<I: I2c>(
        &mut self,
        i2c: &mut I,
        pwm_centidegrees: &[u16],
    ) -> Result<(), SensorError> {
        if self.number_of_servos > 16 {
            return Err(SensorError::InvalidServoCount(self.number_of_servos));
        }
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
