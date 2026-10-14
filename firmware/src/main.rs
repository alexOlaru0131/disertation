#![no_std]
#![no_main]

mod icm20948;
mod pca9685;

use panic_halt as _;

use bsp::ehal::i2c::{ErrorKind, I2c};
use bsp::hal::{clock::GenericClockController, prelude::*, usb::UsbBus};
use bsp::pac;
use feather_m0 as bsp;

use icm20948::{Icm20948, SensorError};
use pca9685::{Pca9685, SensorError as PcaError};
use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};

const UF2_MAGIC_ADDRESS: *mut u32 = 0x2000_7FFC as *mut u32;
const UF2_MAGIC: u32 = 0xF016_69EF;
const PWM_CHANNELS: usize = 10;
const PWM_ANGLES: usize = 2 * PWM_CHANNELS;
const PWM_MAX_ANGLE: u16 = 36_000;
const PWM_ADDRESS_1: u8 = 0x41;
const PWM_ADDRESS_2: u8 = 0x42;

fn poll_usb(
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

fn wait_ms(
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

fn send_packet(
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

fn i2c_error_kind(kind: ErrorKind) -> u8 {
    match kind {
        ErrorKind::NoAcknowledge(_) => 1,
        ErrorKind::Bus => 2,
        ErrorKind::ArbitrationLoss => 3,
        ErrorKind::Overrun => 4,
        _ => 0,
    }
}

fn pwm_error_packet(address: u8, error: PcaError) -> [u8; 5] {
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

fn update_pwm<I: I2c, D: bsp::ehal::delay::DelayNs>(
    pwm: &mut Pca9685,
    initialized: &mut bool,
    i2c: &mut I,
    delay: &mut D,
    angles: &[u16],
) -> Result<(), PcaError> {
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

fn error_packet(error: SensorError) -> [u8; 4] {
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

#[bsp::entry]
fn main() -> ! {
    let mut peripherals = pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();

    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.gclk,
        &mut peripherals.pm,
        &mut peripherals.sysctrl,
        &mut peripherals.nvmctrl,
    );
    let pins = bsp::Pins::new(peripherals.port);

    let mut i2c = bsp::i2c_master(
        &mut clocks,
        100.kHz(),
        peripherals.sercom3,
        &mut peripherals.pm,
        pins.sda,
        pins.scl,
    );

    let usb_bus = bsp::usb_allocator(
        peripherals.usb,
        &mut clocks,
        &mut peripherals.pm,
        pins.usb_dm,
        pins.usb_dp,
    );
    let mut serial = SerialPort::new(&usb_bus);
    let mut usb = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x239A, 0x8023))
        .strings(&[StringDescriptors::default()
            .manufacturer("Alex")
            .product("Software Racheta Debug")
            .serial_number("001")])
        .unwrap()
        .device_class(USB_CLASS_CDC)
        .build();

    let mut delay = bsp::hal::delay::Delay::new(core.SYST, &mut clocks);
    let mut sensor = Icm20948::new();
    let mut pwm_1 = Pca9685::new(PWM_CHANNELS as u8, PWM_ADDRESS_1);
    let mut pwm_2 = Pca9685::new(PWM_CHANNELS as u8, PWM_ADDRESS_2);
    let mut initialized = false;
    let mut pwm_1_initialized = false;
    let mut pwm_2_initialized = false;
    let mut pwm_payload = [0u8; 2 * PWM_ANGLES];
    let mut pwm_received = None;
    let mut bootloader_armed = false;

    wait_ms(
        &mut usb,
        &mut serial,
        &mut delay,
        &mut bootloader_armed,
        1000,
    );

    loop {
        poll_usb(&mut usb, &mut serial, &mut bootloader_armed);
        let mut commands = [0; 64];
        if let Ok(count) = serial.read(&mut commands) {
            for &command in &commands[..count] {
                if let Some(received) = pwm_received {
                    pwm_payload[received] = command;
                    if received + 1 < pwm_payload.len() {
                        pwm_received = Some(received + 1);
                        continue;
                    }
                    pwm_received = None;

                    let mut angles = [0u16; PWM_ANGLES];
                    for (angle, bytes) in angles.iter_mut().zip(pwm_payload.chunks_exact(2)) {
                        *angle = u16::from_le_bytes([bytes[0], bytes[1]]);
                    }
                    if let Some(index) = angles.iter().position(|angle| *angle > PWM_MAX_ANGLE) {
                        let address = if index < PWM_CHANNELS {
                            PWM_ADDRESS_1
                        } else {
                            PWM_ADDRESS_2
                        };
                        send_packet(
                            &mut usb,
                            &mut serial,
                            &mut delay,
                            &mut bootloader_armed,
                            &[0xE1, address, 4, (index % PWM_CHANNELS) as u8, 0],
                        );
                        continue;
                    }

                    let result_1 = update_pwm(
                        &mut pwm_1,
                        &mut pwm_1_initialized,
                        &mut i2c,
                        &mut delay,
                        &angles[..PWM_CHANNELS],
                    );
                    let result_2 = update_pwm(
                        &mut pwm_2,
                        &mut pwm_2_initialized,
                        &mut i2c,
                        &mut delay,
                        &angles[PWM_CHANNELS..],
                    );
                    match (result_1, result_2) {
                        (Err(error), _) => send_packet(
                            &mut usb,
                            &mut serial,
                            &mut delay,
                            &mut bootloader_armed,
                            &pwm_error_packet(PWM_ADDRESS_1, error),
                        ),
                        (_, Err(error)) => send_packet(
                            &mut usb,
                            &mut serial,
                            &mut delay,
                            &mut bootloader_armed,
                            &pwm_error_packet(PWM_ADDRESS_2, error),
                        ),
                        (Ok(()), Ok(())) => send_packet(
                            &mut usb,
                            &mut serial,
                            &mut delay,
                            &mut bootloader_armed,
                            &[0x11, PWM_ANGLES as u8],
                        ),
                    }
                    continue;
                }
                if command == 0x11 {
                    pwm_received = Some(0);
                    continue;
                }
                if command != 0x10 {
                    continue;
                }

                if !initialized {
                    match sensor.init(&mut i2c) {
                        Ok(()) => {
                            initialized = true;
                            wait_ms(&mut usb, &mut serial, &mut delay, &mut bootloader_armed, 15);
                        }
                        Err(error) => {
                            send_packet(
                                &mut usb,
                                &mut serial,
                                &mut delay,
                                &mut bootloader_armed,
                                &error_packet(error),
                            );
                            continue;
                        }
                    }
                }

                for attempt in 0..25 {
                    match sensor.read_measurements(&mut i2c) {
                        Ok(measurements) => {
                            let mut reply = [0; 41];
                            reply[0] = 0x10;
                            for (word, value) in
                                reply[1..].chunks_exact_mut(4).zip(measurements.values)
                            {
                                word.copy_from_slice(&value.to_le_bytes());
                            }
                            send_packet(
                                &mut usb,
                                &mut serial,
                                &mut delay,
                                &mut bootloader_armed,
                                &reply,
                            );
                            break;
                        }
                        Err(SensorError::MagNotReady) if attempt < 24 => {
                            wait_ms(&mut usb, &mut serial, &mut delay, &mut bootloader_armed, 1);
                        }
                        Err(error) => {
                            if !matches!(error, SensorError::MagNotReady | SensorError::MagOverflow)
                            {
                                initialized = false;
                            }
                            send_packet(
                                &mut usb,
                                &mut serial,
                                &mut delay,
                                &mut bootloader_armed,
                                &error_packet(error),
                            );
                            break;
                        }
                    }
                }
            }
        }
        delay.delay_ms(1u32);
    }
}
