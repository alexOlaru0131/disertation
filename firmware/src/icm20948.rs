use i2cdev::{
    core::I2CDevice,
    linux::{LinuxI2CDevice, LinuxI2CError},
};
use std::{fmt, thread, time::Duration};

const WHO_AM_I: u8 = 0x00;
const PWR_MGMT_1: u8 = 0x06;
const PWR_MGMT_2: u8 = 0x07;
const USER_CTRL: u8 = 0x03;
const INT_PIN_CFG: u8 = 0x0F;
const I2C_MST_STATUS: u8 = 0x17;
const ACCEL_XOUT_H: u8 = 0x2D;
const GYRO_CONFIG_1: u8 = 0x01;
const ACCEL_CONFIG: u8 = 0x14;
const REG_BANK_SEL: u8 = 0x7F;
const I2C_MST_ODR_CONFIG: u8 = 0x00;
const I2C_MST_CTRL: u8 = 0x01;
const I2C_MST_DELAY_CTRL: u8 = 0x02;
const I2C_SLV0_ADDR: u8 = 0x03;
const I2C_SLV0_REG: u8 = 0x04;
const I2C_SLV0_CTRL: u8 = 0x05;
const I2C_SLV4_ADDR: u8 = 0x13;
const I2C_SLV4_REG: u8 = 0x14;
const I2C_SLV4_CTRL: u8 = 0x15;
const I2C_SLV4_DO: u8 = 0x16;
const I2C_SLV4_DI: u8 = 0x17;
const MAG_ADDRESS: u8 = 0x0C;
const MAG_WIA2: u8 = 0x01;
const MAG_ST1: u8 = 0x10;
const MAG_CNTL2: u8 = 0x31;
const BANK_0: u8 = 0x00;
const BANK_2: u8 = 0x20;
const BANK_3: u8 = 0x30;
const EXPECTED_ID: u8 = 0xEA;
const ACCEL_16G: u8 = 0x06;
const GYRO_2000DPS: u8 = 0x06;

pub struct Measurements {
    pub values: [i32; 10],
}

#[derive(Debug)]
pub enum SensorError {
    I2c(&'static str, LinuxI2CError),
    WrongId(u8),
    WrongBank(u8),
    WrongAccelConfig(u8),
    WrongGyroConfig(u8),
    WrongMagId(u8),
    MagNoAck(u8),
    MagLostArbitration(u8),
    MagTimeout,
    MagNotReady,
    MagOverflow,
    ShortRead(&'static str, usize),
}

impl fmt::Display for SensorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for SensorError {}

pub struct Icm20948 {
    i2c_imu: LinuxI2CDevice,
}

impl Icm20948 {
    pub fn new() -> Result<Self, SensorError> {
        Ok(Self {
            i2c_imu: LinuxI2CDevice::new("/dev/i2c-1", 0x69)
                .map_err(|e| SensorError::I2c("open IMU", e))?,
        })
    }

    fn select_bank(&mut self, bank: u8) -> Result<(), SensorError> {
        self.i2c_imu
            .smbus_write_byte_data(REG_BANK_SEL, bank)
            .map_err(|e| SensorError::I2c("select bank", e))
    }

    fn read_imu_reg(&mut self, reg: u8) -> Result<u8, SensorError> {
        self.i2c_imu
            .smbus_read_byte_data(reg)
            .map_err(|e| SensorError::I2c("read IMU register", e))
    }

    fn write_imu_reg(
        &mut self,
        reg: u8,
        value: u8,
        stage: &'static str,
    ) -> Result<(), SensorError> {
        self.i2c_imu
            .smbus_write_byte_data(reg, value)
            .map_err(|e| SensorError::I2c(stage, e))
    }

    fn mag_transaction(&mut self, reg: u8, write: Option<u8>) -> Result<u8, SensorError> {
        let _ = self.read_imu_reg(I2C_MST_STATUS)?;
        self.select_bank(BANK_3)?;
        let setup = (|| {
            if let Some(value) = write {
                self.write_imu_reg(I2C_SLV4_DO, value, "aux write data")?;
            }
            let address = MAG_ADDRESS | if write.is_none() { 0x80 } else { 0 };
            self.write_imu_reg(I2C_SLV4_ADDR, address, "aux slave address")?;
            self.write_imu_reg(I2C_SLV4_REG, reg, "aux register")?;
            self.write_imu_reg(I2C_SLV4_CTRL, 0x80, "aux start transaction")
        })();
        let restore = self.select_bank(BANK_0);
        setup?;
        restore?;

        for _ in 0..50 {
            let status = self.read_imu_reg(I2C_MST_STATUS)?;
            if status & 0x10 != 0 {
                return Err(SensorError::MagNoAck(status));
            }
            if status & 0x20 != 0 {
                return Err(SensorError::MagLostArbitration(status));
            }
            if status & 0x40 != 0 {
                if write.is_some() {
                    return Ok(0);
                }
                self.select_bank(BANK_3)?;
                let value = self.read_imu_reg(I2C_SLV4_DI);
                let restore = self.select_bank(BANK_0);
                let value = value?;
                restore?;
                return Ok(value);
            }
            thread::sleep(Duration::from_millis(1));
        }
        Err(SensorError::MagTimeout)
    }

    pub fn init(&mut self) -> Result<(), SensorError> {
        self.select_bank(BANK_0)?;

        let id = self.read_imu_reg(WHO_AM_I)?;
        if id != EXPECTED_ID {
            return Err(SensorError::WrongId(id));
        }

        self.i2c_imu
            .smbus_write_byte_data(PWR_MGMT_1, 0x01)
            .map_err(|e| SensorError::I2c("wake", e))?;
        self.i2c_imu
            .smbus_write_byte_data(PWR_MGMT_2, 0x00)
            .map_err(|e| SensorError::I2c("enable axes", e))?;

        self.select_bank(BANK_2)?;
        let config = (|| {
            let selected = self.read_imu_reg(REG_BANK_SEL)?;
            if selected & 0x30 != BANK_2 {
                return Err(SensorError::WrongBank(selected));
            }

            self.i2c_imu
                .smbus_write_byte_data(ACCEL_CONFIG, ACCEL_16G)
                .map_err(|e| SensorError::I2c("accel config", e))?;
            let setting = self.read_imu_reg(ACCEL_CONFIG)?;
            if setting & 0x06 != ACCEL_16G {
                return Err(SensorError::WrongAccelConfig(setting));
            }

            self.i2c_imu
                .smbus_write_byte_data(GYRO_CONFIG_1, GYRO_2000DPS)
                .map_err(|e| SensorError::I2c("gyro config", e))?;

            let setting = self.read_imu_reg(GYRO_CONFIG_1)?;
            if setting & 0x06 != GYRO_2000DPS {
                return Err(SensorError::WrongGyroConfig(setting));
            }
            Ok(())
        })();
        let restore = self.select_bank(BANK_0);
        config?;
        restore?;

        self.write_imu_reg(INT_PIN_CFG, 0x00, "disable aux bypass")?;
        self.write_imu_reg(USER_CTRL, 0x20, "enable aux master")?;
        self.select_bank(BANK_3)?;
        let setup = self.write_imu_reg(I2C_MST_CTRL, 0x17, "aux master clock");
        let restore = self.select_bank(BANK_0);
        setup?;
        restore?;

        let mag_id = self.mag_transaction(MAG_WIA2, None)?;
        if mag_id != 0x09 {
            return Err(SensorError::WrongMagId(mag_id));
        }
        self.mag_transaction(MAG_CNTL2, Some(0x00))?;
        thread::sleep(Duration::from_millis(2));
        self.mag_transaction(MAG_CNTL2, Some(0x08))?;

        self.select_bank(BANK_3)?;
        let setup = (|| {
            self.write_imu_reg(I2C_MST_ODR_CONFIG, 0x05, "aux sample rate")?;
            self.write_imu_reg(I2C_MST_DELAY_CTRL, 0x01, "aux slave delay")?;
            self.write_imu_reg(I2C_SLV0_ADDR, 0x80 | MAG_ADDRESS, "aux read address")?;
            self.write_imu_reg(I2C_SLV0_REG, MAG_ST1, "aux read register")?;
            self.write_imu_reg(I2C_SLV0_CTRL, 0x80 | 9, "aux read length")
        })();
        let restore = self.select_bank(BANK_0);
        setup?;
        restore?;
        thread::sleep(Duration::from_millis(40));
        Ok(())
    }

    pub fn read_measurements(&mut self) -> Result<Measurements, SensorError> {
        let frame = self
            .i2c_imu
            .smbus_read_i2c_block_data(ACCEL_XOUT_H, 23)
            .map_err(|e| SensorError::I2c("read accel/gyro/temp", e))?;
        if frame.len() != 23 {
            return Err(SensorError::ShortRead("measurements", frame.len()));
        }
        let data = &frame[..14];
        let mag_data = &frame[14..];
        if mag_data[0] & 1 == 0 {
            return Err(SensorError::MagNotReady);
        }
        if mag_data[8] & 0x08 != 0 {
            return Err(SensorError::MagOverflow);
        }

        let accel = |i| i16::from_be_bytes([data[i], data[i + 1]]) as i64;
        let gyro = |i| i16::from_be_bytes([data[i], data[i + 1]]) as i64;
        let magnetic = |i| i16::from_le_bytes([mag_data[i], mag_data[i + 1]]) as i32;
        let temp = i16::from_be_bytes([data[12], data[13]]) as i64;

        Ok(Measurements {
            values: [
                accel_x100(accel(0)),
                accel_x100(accel(2)),
                accel_x100(accel(4)),
                gyro_x100(gyro(6)),
                gyro_x100(gyro(8)),
                gyro_x100(gyro(10)),
                magnetic(1) * 15,
                magnetic(3) * 15,
                magnetic(5) * 15,
                2_100 + round_div(temp * 10_000, 33_387),
            ],
        })
    }
}

fn round_div(n: i64, d: i64) -> i32 {
    ((n + if n >= 0 { d / 2 } else { -(d / 2) }) / d) as i32
}

fn accel_x100(raw: i64) -> i32 {
    round_div(raw * 980_665, 2_048_000)
}

fn gyro_x100(raw: i64) -> i32 {
    round_div(raw * 3_141_593_000, 29_520_000_000)
}
