use crate::{
    UF2_MAGIC, UF2_MAGIC_ADDRESS, bsp, icm20948::SensorError, pca9685::SensorError as PcaError,
};
use bsp::ehal::{delay::DelayNs as _, i2c::ErrorKind};
use bsp::hal::usb::UsbBus;
use usb_device::prelude::UsbDevice;
use usbd_serial::SerialPort;

pub(crate) fn poll_usb(
    usb: &mut UsbDevice<'_, UsbBus>,
    serial: &mut SerialPort<'_, UsbBus>,
    armed: &mut bool,
) {
    usb.poll(&mut [serial]);

    if serial.line_coding().data_rate() == 1200 {
        if serial.dtr() {
            *armed = true;
        } else if *armed {
            unsafe { core::ptr::write_volatile(UF2_MAGIC_ADDRESS, UF2_MAGIC) };
            cortex_m::peripheral::SCB::sys_reset();
        }
    } else {
        *armed = false;
    }
}

pub(crate) fn wait_ms(
    usb: &mut UsbDevice<'_, UsbBus>,
    serial: &mut SerialPort<'_, UsbBus>,
    delay: &mut bsp::hal::delay::Delay,
    armed: &mut bool,
    ms: u32,
) {
    for _ in 0..ms {
        poll_usb(usb, serial, armed);
        delay.delay_ms(1u32);
    }
}

pub(crate) fn send_packet(
    usb: &mut UsbDevice<'_, UsbBus>,
    serial: &mut SerialPort<'_, UsbBus>,
    delay: &mut bsp::hal::delay::Delay,
    armed: &mut bool,
    data: &[u8],
) {
    let mut pending = data;
    for _ in 0..50 {
        if pending.is_empty() {
            break;
        }
        poll_usb(usb, serial, armed);
        if let Ok(count) = serial.write(pending) {
            pending = &pending[count..];
        }
        delay.delay_ms(1u32);
    }
}

pub(crate) fn error_packet(error: SensorError) -> [u8; 4] {
    match error {
        SensorError::I2c(stage, kind) => {
            let stage = match stage {
                "select bank" => 1,
                "read register" => 2,
                "wake" => 3,
                "enable axes" => 4,
                "accel config" => 5,
                "gyro config" => 6,
                "disable aux master" => 7,
                "mag bypass" => 8,
                "mag identity" => 9,
                "mag continuous" => 10,
                "read accel/gyro/temp" => 11,
                "read magnetometer" => 12,
                _ => 0,
            };
            [0xE0, 1, stage, i2c_error_kind(kind)]
        }
        SensorError::WrongId(id) => [0xE0, 2, id, 0],
        SensorError::WrongBank(bank) => [0xE0, 3, bank, 0],
        SensorError::WrongAccelConfig(setting) => [0xE0, 4, setting, 0],
        SensorError::WrongMagId(id) => [0xE0, 5, id, 0],
        SensorError::MagNotReady => [0xE0, 6, 0, 0],
        SensorError::MagOverflow => [0xE0, 7, 0, 0],
        SensorError::WrongGyroConfig(setting) => [0xE0, 8, setting, 0],
    }
}

pub(crate) fn pwm_error_packet(address: u8, error: PcaError) -> [u8; 5] {
    match error {
        PcaError::I2c(stage, kind) => {
            let stage = match stage {
                "read register" => 1,
                "mode2 outdrv" => 2,
                "mode1 sleep" => 3,
                "pre scale" => 4,
                "mode1 wake" => 5,
                "mode1 restart" => 6,
                "set pwm" => 7,
                _ => 0,
            };
            [0xE1, address, 1, stage, i2c_error_kind(kind)]
        }
        PcaError::InvalidServoCount(count) => [0xE1, address, 2, count, 0],
        PcaError::InvalidPwmCount(count) => [0xE1, address, 3, count as u8, 0],
    }
}

fn i2c_error_kind(kind: ErrorKind) -> u8 {
    match kind {
        ErrorKind::NoAcknowledge(_) => 1,
        ErrorKind::Bus => 2,
        ErrorKind::ArbitrationLoss => 3,
        ErrorKind::Overrun => 4,
        _ => 0,
    }
}
