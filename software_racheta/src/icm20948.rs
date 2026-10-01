use feather_m0::ehal::i2c::{Error as _, ErrorKind, I2c};

const WHO_AM_I: u8 = 0x00;
const PWR_MGMT_1: u8 = 0x06;
const PWR_MGMT_2: u8 = 0x07;
const USER_CTRL: u8 = 0x03;
const INT_PIN_CFG: u8 = 0x0F;
const ACCEL_XOUT_H: u8 = 0x2D;
const GYRO_CONFIG_1: u8 = 0x01;
const ACCEL_CONFIG: u8 = 0x14;
const REG_BANK_SEL: u8 = 0x7F;
const MAG_ADDRESS: u8 = 0x0C;
const MAG_WIA2: u8 = 0x01;
const MAG_ST1: u8 = 0x10;
const MAG_CNTL2: u8 = 0x31;

const BANK_0: u8 = 0x00;
const BANK_2: u8 = 0x20;
const EXPECTED_ID: u8 = 0xEA;
const ACCEL_16G: u8 = 0x06;
const GYRO_2000DPS: u8 = 0x06;

pub struct Measurements {
    pub values: [i32; 10],
}

#[derive(Debug)]
pub enum SensorError {
    I2c(&'static str, ErrorKind),
    WrongId(u8),
    WrongBank(u8),
    WrongAccelConfig(u8),
    WrongGyroConfig(u8),
    WrongMagId(u8),
    MagNotReady,
    MagOverflow,
}

pub struct Icm20948 {
    address: u8,
}

impl Icm20948 {
    pub fn new() -> Self {
        Self { address: 0x69 }
    }

    fn select_bank<I: I2c>(&self, i2c: &mut I, bank: u8) -> Result<(), SensorError> {
        i2c.write(self.address, &[REG_BANK_SEL, bank])
            .map_err(|e| SensorError::I2c("select bank", e.kind()))
    }

    fn read_reg<I: I2c>(&self, i2c: &mut I, reg: u8) -> Result<u8, SensorError> {
        let mut data = [0];
        i2c.write_read(self.address, &[reg], &mut data)
            .map_err(|e| SensorError::I2c("read register", e.kind()))?;
        Ok(data[0])
    }

    pub fn init<I: I2c>(&mut self, i2c: &mut I) -> Result<(), SensorError> {
        self.select_bank(i2c, BANK_0)?;

        let id = self.read_reg(i2c, WHO_AM_I)?;
        if id != EXPECTED_ID {
            return Err(SensorError::WrongId(id));
        }

        i2c.write(self.address, &[PWR_MGMT_1, 0x01])
            .map_err(|e| SensorError::I2c("wake", e.kind()))?;
        i2c.write(self.address, &[PWR_MGMT_2, 0x00])
            .map_err(|e| SensorError::I2c("enable axes", e.kind()))?;

        self.select_bank(i2c, BANK_2)?;
        let config = (|| {
            let selected = self.read_reg(i2c, REG_BANK_SEL)?;
            if selected & 0x30 != BANK_2 {
                return Err(SensorError::WrongBank(selected));
            }

            i2c.write(self.address, &[ACCEL_CONFIG, ACCEL_16G])
                .map_err(|e| SensorError::I2c("accel config", e.kind()))?;
            let setting = self.read_reg(i2c, ACCEL_CONFIG)?;
            if setting & 0x06 != ACCEL_16G {
                return Err(SensorError::WrongAccelConfig(setting));
            }

            i2c.write(self.address, &[GYRO_CONFIG_1, GYRO_2000DPS])
                .map_err(|e| SensorError::I2c("gyro config", e.kind()))?;
            let setting = self.read_reg(i2c, GYRO_CONFIG_1)?;
            if setting & 0x06 != GYRO_2000DPS {
                return Err(SensorError::WrongGyroConfig(setting));
            }
            Ok(())
        })();
        let restore = self.select_bank(i2c, BANK_0);
        config?;
        restore?;

        i2c.write(self.address, &[USER_CTRL, 0x00])
            .map_err(|e| SensorError::I2c("disable aux master", e.kind()))?;
        i2c.write(self.address, &[INT_PIN_CFG, 0x02])
            .map_err(|e| SensorError::I2c("mag bypass", e.kind()))?;

        let mut id = [0];
        i2c.write_read(MAG_ADDRESS, &[MAG_WIA2], &mut id)
            .map_err(|e| SensorError::I2c("mag identity", e.kind()))?;
        if id[0] != 0x09 {
            return Err(SensorError::WrongMagId(id[0]));
        }
        i2c.write(MAG_ADDRESS, &[MAG_CNTL2, 0x08])
            .map_err(|e| SensorError::I2c("mag continuous", e.kind()))?;
        Ok(())
    }

    pub fn read_measurements<I: I2c>(&mut self, i2c: &mut I) -> Result<Measurements, SensorError> {
        let mut data = [0; 14];
        i2c.write_read(self.address, &[ACCEL_XOUT_H], &mut data)
            .map_err(|e| SensorError::I2c("read accel/gyro/temp", e.kind()))?;

        let mut mag = [0; 9];
        i2c.write_read(MAG_ADDRESS, &[MAG_ST1], &mut mag)
            .map_err(|e| SensorError::I2c("read magnetometer", e.kind()))?;
        if mag[0] & 1 == 0 {
            return Err(SensorError::MagNotReady);
        }
        if mag[8] & 0x08 != 0 {
            return Err(SensorError::MagOverflow);
        }

        let accel = |i| i16::from_be_bytes([data[i], data[i + 1]]) as i64;
        let gyro = |i| i16::from_be_bytes([data[i], data[i + 1]]) as i64;
        let magnetic = |i| i16::from_le_bytes([mag[i], mag[i + 1]]) as i32;
        let temp = i16::from_be_bytes([data[12], data[13]]) as i64;

        Ok(Measurements {
            values: [
                accel_x100(accel(0)),
                accel_x100(accel(2)),
                accel_x100(accel(4)),
                gyro_x100(gyro(6)),
                gyro_x100(gyro(8)),
                gyro_x100(gyro(10)),
                magnetic(1) * 15, // 0.15 µT/LSB * 100
                magnetic(3) * 15,
                magnetic(5) * 15,
                2_100 + round_div(temp * 10_000, 33_387), // 21°C la TEMP_OUT = 0
            ],
        })
    }
}

fn round_div(n: i64, d: i64) -> i32 {
    ((n + if n >= 0 { d / 2 } else { -(d / 2) }) / d) as i32
}

fn accel_x100(raw: i64) -> i32 {
    // ±16 g: 2048 LSB/g; 1 g = 9.80665 m/s².
    round_div(raw * 980_665, 2_048_000)
}

fn gyro_x100(raw: i64) -> i32 {
    // ±2000 °/s: 16.4 LSB/(°/s); convertire °/s -> rad/s.
    // π aproximat la 3.141593, cu precizie sub o sutime in acest domeniu.
    round_div(raw * 3_141_593_000, 29_520_000_000)
}
