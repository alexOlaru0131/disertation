#![no_std]
#![no_main]

mod icm20948;
mod pca9685;
mod usb;

use panic_halt as _;

use bsp::ehal::i2c::I2c;
use bsp::hal::gpio::{PA02, Pin, PushPullOutput};
use bsp::hal::{clock::GenericClockController, prelude::*, usb::UsbBus};
use feather_m0 as bsp;

use icm20948::{Icm20948, SensorError};
use pca9685::*;
use usb::*;
use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};

const UF2_MAGIC_ADDRESS: *mut u32 = 0x2000_7FFC as *mut u32;
const UF2_MAGIC: u32 = 0xF016_69EF;
const PWM_CHANNELS: usize = 10;
const PWM_ANGLES: usize = 2 * PWM_CHANNELS;
const PWM_MAX_ANGLE: u16 = 36_000;
const PWM_ADDRESS_1: u8 = 0x41;
const PWM_ADDRESS_2: u8 = 0x42;
const INA219_ADDRESS: u8 = 0x40;
bsp::hal::rtc_monotonic!(Mono, bsp::hal::rtc::rtic::rtc_clock::Clock32k);

#[rtic::app(device = bsp::pac, dispatchers = [TC4, TC5], peripherals = true)]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        clocks: GenericClockController,
        i2c: bsp::I2c,
        serial: SerialPort<'static, UsbBus>,
        usb: UsbDevice<'static, UsbBus>,
        delay: bsp::hal::delay::Delay,
        bootloader_armed: bool,
        request_sensor: u8,
        request_pwm: Option<[u16; PWM_ANGLES]>,
        request_led: Option<bool>,
    }

    #[local]
    struct Local {
        led: Pin<PA02, PushPullOutput>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        let mut peripherals = cx.device;
        let mut clocks = GenericClockController::with_internal_32kosc(
            peripherals.gclk,
            &mut peripherals.pm,
            &mut peripherals.sysctrl,
            &mut peripherals.nvmctrl,
        );
        let rtc_clock = clocks.gclk1();
        clocks.rtc(&rtc_clock).unwrap();
        Mono::start(peripherals.rtc);

        let pins = bsp::Pins::new(peripherals.port);
        let mut led = pins.a0.into_push_pull_output();
        let mut voltage = 0;
        led.set_low().unwrap();
        let i2c = bsp::i2c_master(
            &mut clocks,
            100.kHz(),
            peripherals.sercom3,
            &mut peripherals.pm,
            pins.sda,
            pins.scl,
        );

        let usb_bus: &'static UsbBusAllocator<UsbBus> = cortex_m::singleton!(
            : UsbBusAllocator<UsbBus> = bsp::usb_allocator(
                peripherals.usb,
                &mut clocks,
                &mut peripherals.pm,
                pins.usb_dm,
                pins.usb_dp,
            )
        )
        .unwrap();
        let serial = SerialPort::new(usb_bus);
        let usb = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x239A, 0x8023))
            .strings(&[StringDescriptors::default()
                .manufacturer("Alex")
                .product("Software Racheta Debug")
                .serial_number("001")])
            .unwrap()
            .device_class(USB_CLASS_CDC)
            .build();
        let delay = bsp::hal::delay::Delay::new(cx.core.SYST, &mut clocks);

        usb_task::spawn().unwrap();
        icm20948_task::spawn().unwrap();
        pca9685_task::spawn().unwrap();
        led_task::spawn().unwrap();

        (
            Shared {
                clocks,
                i2c,
                serial,
                usb,
                delay,
                bootloader_armed: false,
                request_sensor: 0,
                request_pwm: None,
                request_led: None,
            },
            Local { led },
        )
    }

    #[task(priority = 2, shared = [usb, serial, delay, bootloader_armed, request_sensor, request_pwm, request_led])]
    async fn usb_task(mut cx: usb_task::Context) {
        let mut pwm_payload = [0u8; 2 * PWM_ANGLES];
        let mut pwm_received = None;

        (
            &mut cx.shared.usb,
            &mut cx.shared.serial,
            &mut cx.shared.delay,
            &mut cx.shared.bootloader_armed,
        )
            .lock(|usb, serial, delay, armed| wait_ms(usb, serial, delay, armed, 1000));

        loop {
            let mut commands = [0; 64];
            let count = (
                &mut cx.shared.usb,
                &mut cx.shared.serial,
                &mut cx.shared.bootloader_armed,
            )
                .lock(|usb, serial, armed| {
                    poll_usb(usb, serial, armed);
                    serial.read(&mut commands).unwrap_or(0)
                });

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
                        (
                            &mut cx.shared.usb,
                            &mut cx.shared.serial,
                            &mut cx.shared.delay,
                            &mut cx.shared.bootloader_armed,
                        )
                            .lock(|usb, serial, delay, armed| {
                                send_packet(
                                    usb,
                                    serial,
                                    delay,
                                    armed,
                                    &[0xE1, address, 4, (index % PWM_CHANNELS) as u8, 0],
                                )
                            });
                        continue;
                    }

                    while !cx.shared.request_pwm.lock(|pending| {
                        if pending.is_some() {
                            false
                        } else {
                            *pending = Some(angles);
                            true
                        }
                    }) {
                        Mono::delay(5u64.millis()).await;
                    }
                    continue;
                }
                if command == 0x20 {
                    pwm_received = Some(0);
                } else if command == 0x10 {
                    cx.shared
                        .request_sensor
                        .lock(|requested| *requested = requested.saturating_add(1));
                } else if command == 0x30 || command == 0x35 {
                    cx.shared
                        .request_led
                        .lock(|pending| *pending = Some(command == 0x30));
                }
            }
            Mono::delay(5u64.millis()).await;
        }
    }

    #[task(priority = 1, shared = [request_sensor, i2c, usb, serial, delay, bootloader_armed])]
    async fn icm20948_task(mut cx: icm20948_task::Context) {
        let mut sensor = Icm20948::new();
        let mut initialized = false;

        loop {
            let requested = cx.shared.request_sensor.lock(|requested| {
                if *requested == 0 {
                    false
                } else {
                    *requested -= 1;
                    true
                }
            });
            if !requested {
                Mono::delay(1u64.millis()).await;
                continue;
            }

            if !initialized {
                match cx.shared.i2c.lock(|i2c| sensor.init(i2c)) {
                    Ok(()) => {
                        initialized = true;
                        (
                            &mut cx.shared.usb,
                            &mut cx.shared.serial,
                            &mut cx.shared.delay,
                            &mut cx.shared.bootloader_armed,
                        )
                            .lock(|usb, serial, delay, armed| {
                                wait_ms(usb, serial, delay, armed, 15)
                            });
                    }
                    Err(error) => {
                        (
                            &mut cx.shared.usb,
                            &mut cx.shared.serial,
                            &mut cx.shared.delay,
                            &mut cx.shared.bootloader_armed,
                        )
                            .lock(|usb, serial, delay, armed| {
                                send_packet(usb, serial, delay, armed, &error_packet(error))
                            });
                        continue;
                    }
                }
            }

            for attempt in 0..25 {
                match cx.shared.i2c.lock(|i2c| sensor.read_measurements(i2c)) {
                    Ok(measurements) => {
                        let mut reply = [0; 41];
                        reply[0] = 0x10;
                        for (word, value) in reply[1..].chunks_exact_mut(4).zip(measurements.values)
                        {
                            word.copy_from_slice(&value.to_le_bytes());
                        }
                        (
                            &mut cx.shared.usb,
                            &mut cx.shared.serial,
                            &mut cx.shared.delay,
                            &mut cx.shared.bootloader_armed,
                        )
                            .lock(|usb, serial, delay, armed| {
                                send_packet(usb, serial, delay, armed, &reply)
                            });
                        break;
                    }
                    Err(SensorError::MagNotReady) if attempt < 24 => {
                        (
                            &mut cx.shared.usb,
                            &mut cx.shared.serial,
                            &mut cx.shared.delay,
                            &mut cx.shared.bootloader_armed,
                        )
                            .lock(|usb, serial, delay, armed| {
                                wait_ms(usb, serial, delay, armed, 1)
                            });
                    }
                    Err(error) => {
                        if !matches!(error, SensorError::MagNotReady | SensorError::MagOverflow) {
                            initialized = false;
                        }
                        (
                            &mut cx.shared.usb,
                            &mut cx.shared.serial,
                            &mut cx.shared.delay,
                            &mut cx.shared.bootloader_armed,
                        )
                            .lock(|usb, serial, delay, armed| {
                                send_packet(usb, serial, delay, armed, &error_packet(error))
                            });
                        break;
                    }
                }
            }
        }
    }

    #[task(priority = 1, shared = [request_pwm, i2c, delay, usb, serial, bootloader_armed])]
    async fn pca9685_task(mut cx: pca9685_task::Context) {
        let mut pwm_1 = Pca9685::new(PWM_CHANNELS as u8, PWM_ADDRESS_1);
        let mut pwm_2 = Pca9685::new(PWM_CHANNELS as u8, PWM_ADDRESS_2);
        let mut pwm_1_initialized = false;
        let mut pwm_2_initialized = false;

        loop {
            let Some(angles) = cx.shared.request_pwm.lock(|pending| pending.take()) else {
                Mono::delay(5u64.millis()).await;
                continue;
            };

            let (result_1, result_2) =
                (&mut cx.shared.i2c, &mut cx.shared.delay).lock(|i2c, delay| {
                    (
                        update_pwm(
                            &mut pwm_1,
                            &mut pwm_1_initialized,
                            i2c,
                            delay,
                            &angles[..PWM_CHANNELS],
                        ),
                        update_pwm(
                            &mut pwm_2,
                            &mut pwm_2_initialized,
                            i2c,
                            delay,
                            &angles[PWM_CHANNELS..],
                        ),
                    )
                });

            let response = match (result_1, result_2) {
                (Err(error), _) => &pwm_error_packet(PWM_ADDRESS_1, error)[..],
                (_, Err(error)) => &pwm_error_packet(PWM_ADDRESS_2, error)[..],
                (Ok(()), Ok(())) => &[0x20, PWM_ANGLES as u8][..],
            };
            (
                &mut cx.shared.usb,
                &mut cx.shared.serial,
                &mut cx.shared.delay,
                &mut cx.shared.bootloader_armed,
            )
                .lock(|usb, serial, delay, armed| send_packet(usb, serial, delay, armed, response));
        }
    }

    #[task(priority = 1, shared = [request_led], local = [led])]
    async fn led_task(mut cx: led_task::Context) {
        loop {
            match cx.shared.request_led.lock(|pending| pending.take()) {
                Some(true) => cx.local.led.set_high().unwrap(),
                Some(false) => cx.local.led.set_low().unwrap(),
                None => Mono::delay(5u64.millis()).await,
            }
        }
    }
}
