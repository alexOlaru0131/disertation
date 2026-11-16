use i2cdev::{
    core::I2CDevice,
    linux::{LinuxI2CDevice, LinuxI2CError as Error},
};

const WHO_AM_I: u8 = 0x00;
const PWR_MGMT_1: u8 = 0x06;
const PWR_MGMT_2: u8 = 0x07;
const USER_CTRL: u8 = 0x03;
const INT_PIN_CFG: u8 = 0x0F;
const ACCEL_XOUT_H: u8 = 0x2D;
const GYRO_CONFIG_1: u8 = 0x01;
const ACCEL_CONFIG: u8 = 0x14;
const REG_BANK_SEL: u8 = 0x7F;
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
    I2c(&'static str, Error),
    WrongId(u8),
    WrongBank(u8),
    WrongAccelConfig(u8),
    WrongGyroConfig(u8),
    WrongMagId(u8),
    MagNotReady,
    MagOverflow,
}

pub struct Icm20948 {
    i2c_imu: LinuxI2CDevice,
    i2c_mag: LinuxI2CDevice,
}

impl Default for Icm20948 {
    fn default() -> Self {
        Self {
            i2c_imu: LinuxI2CDevice::new("/dev/i2c-1", 0x69).expect("IMU"),
            i2c_mag: LinuxI2CDevice::new("/dev/i2c-1", 0x0C).expect("MAG"),
        }
    }
}

impl Icm20948 {
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

    fn read_mag_reg(&mut self, reg: u8) -> Result<u8, SensorError> {
        self.i2c_mag
            .smbus_read_byte_data(reg)
            .map_err(|e| SensorError::I2c("read magnetometer register", e))
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

        self.i2c_imu
            .smbus_write_byte_data(USER_CTRL, 0x00)
            .map_err(|e| SensorError::I2c("disable aux master", e))?;
        self.i2c_imu
            .smbus_write_byte_data(INT_PIN_CFG, 0x02)
            .map_err(|e| SensorError::I2c("mag bypass", e))?;
        let mag_id = self.read_mag_reg(MAG_WIA2)?;
        if mag_id != 0x09 {
            return Err(SensorError::WrongMagId(mag_id));
        }
        self.i2c_mag
            .smbus_write_byte_data(MAG_CNTL2, 0x08)
            .map_err(|e| SensorError::I2c("mag continuous", e))?;
        Ok(())
    }

    pub fn read_measurements(&mut self) -> Result<Measurements, SensorError> {
        let data = self
            .i2c_imu
            .smbus_read_i2c_block_data(ACCEL_XOUT_H, 14)
            .map_err(|e| SensorError::I2c("read accel/gyro/temp", e))?;

        let mag_data = self
            .i2c_mag
            .smbus_read_i2c_block_data(MAG_ST1, 9)
            .map_err(|e| SensorError::I2c("read magnetometer", e))?;
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
