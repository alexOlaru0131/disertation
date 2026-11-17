use gpio_cdev::{Chip, LineHandle, LineRequestFlags};
use icm20948::Icm20948;
use pca9685::Pca9685;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{mpsc, oneshot};

pub mod icm20948;
pub mod pca9685;
pub mod usb;

pub enum Command {
    ReadImu(oneshot::Sender<Result<[i32; 10], String>>),
    SetPwm([u16; 20], oneshot::Sender<Result<(), String>>),
    ReadStops(oneshot::Sender<Result<[u8; 6], String>>),
    TurnOnLaserLed(oneshot::Sender<Result<(), String>>),
    TurnOffLaserLed(oneshot::Sender<Result<(), String>>),
    TurnOnLed(oneshot::Sender<Result<(), String>>),
    TurnOffLed(oneshot::Sender<Result<(), String>>),
    Shutdown,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (tx, mut rx) = mpsc::channel::<Command>(32);

    let mut i2c_thread = tokio::task::spawn_blocking(
        move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let mut imu = Icm20948::new()?;
            imu.init()?;
            let mut pca42 = Pca9685::new(0x42)?;
            pca42.init()?;
            let mut pca41 = Pca9685::new(0x41)?;
            pca41.init()?;

            let mut chip = Chip::new("/dev/gpiochip0")?;
            let laser_led: LineHandle =
                chip.get_line(26)?
                    .request(LineRequestFlags::OUTPUT, 0, "hexapod-gpio")?;
            let led: LineHandle =
                chip.get_line(16)?
                    .request(LineRequestFlags::OUTPUT, 0, "hexapod-gpio")?;

            let stop_1: LineHandle = chip.get_line(23)?.request(
                LineRequestFlags::INPUT | LineRequestFlags::from_bits_retain(1 << 5),
                0,
                "hexapod-gpio",
            )?;
            let stop_2: LineHandle = chip.get_line(24)?.request(
                LineRequestFlags::INPUT | LineRequestFlags::from_bits_retain(1 << 5),
                0,
                "hexapod-gpio",
            )?;
            let stop_3: LineHandle = chip.get_line(25)?.request(
                LineRequestFlags::INPUT | LineRequestFlags::from_bits_retain(1 << 5),
                0,
                "hexapod-gpio",
            )?;
            let stop_4: LineHandle = chip.get_line(17)?.request(
                LineRequestFlags::INPUT | LineRequestFlags::from_bits_retain(1 << 5),
                0,
                "hexapod-gpio",
            )?;
            let stop_5: LineHandle = chip.get_line(27)?.request(
                LineRequestFlags::INPUT | LineRequestFlags::from_bits_retain(1 << 5),
                0,
                "hexapod-gpio",
            )?;
            let stop_6: LineHandle = chip.get_line(22)?.request(
                LineRequestFlags::INPUT | LineRequestFlags::from_bits_retain(1 << 5),
                0,
                "hexapod-gpio",
            )?;

            while let Some(command) = rx.blocking_recv() {
                match command {
                    Command::ReadImu(reply) => {
                        let result = imu
                            .read_measurements()
                            .map(|measurements| measurements.values)
                            .map_err(|error| error.to_string());
                        let _ = reply.send(result);
                    }
                    Command::SetPwm(angles, reply) => {
                        let mut errors = Vec::new();
                        if let Err(error) = pca42.set_pwm_vals(&angles[..10]) {
                            errors.push(format!("PCA 0x42: {error}"));
                        }
                        if let Err(error) = pca41.set_pwm_vals(&angles[10..]) {
                            errors.push(format!("PCA 0x41: {error}"));
                        }
                        let result = if errors.is_empty() {
                            Ok(())
                        } else {
                            Err(errors.join("; "))
                        };
                        let _ = reply.send(result);
                    }
                    Command::TurnOnLaserLed(reply) => match laser_led.set_value(1u8) {
                        Ok(()) => {
                            let _ = reply.send(Ok(()));
                        }
                        Err(error) => {
                            let _ = reply.send(Err(error.to_string()));
                        }
                    },
                    Command::TurnOffLaserLed(reply) => match laser_led.set_value(0u8) {
                        Ok(()) => {
                            let _ = reply.send(Ok(()));
                        }
                        Err(error) => {
                            let _ = reply.send(Err(error.to_string()));
                        }
                    },
                    Command::TurnOnLed(reply) => match led.set_value(1u8) {
                        Ok(()) => {
                            let _ = reply.send(Ok(()));
                        }
                        Err(error) => {
                            let _ = reply.send(Err(error.to_string()));
                        }
                    },
                    Command::TurnOffLed(reply) => match led.set_value(0u8) {
                        Ok(()) => {
                            let _ = reply.send(Ok(()));
                        }
                        Err(error) => {
                            let _ = reply.send(Err(error.to_string()));
                        }
                    },
                    Command::ReadStops(reply) => {
                        let _ = reply.send(Ok([
                            stop_1.get_value()?,
                            stop_2.get_value()?,
                            stop_3.get_value()?,
                            stop_4.get_value()?,
                            stop_5.get_value()?,
                            stop_6.get_value()?,
                        ]));
                    }
                    Command::Shutdown => break,
                }
            }

            Ok(())
        },
    );

    let stop = Arc::new(AtomicBool::new(false));
    let usb_thread = std::thread::spawn({
        let stop = Arc::clone(&stop);
        let tx = tx.clone();
        move || usb::serve(tx, stop)
    });

    let result = tokio::select! {
        result = &mut i2c_thread => result?,
        signal = tokio::signal::ctrl_c() => {
            signal?;
            stop.store(true, Ordering::Relaxed);
            let _ = tx.send(Command::Shutdown).await;
            i2c_thread.await?
        }
    };
    stop.store(true, Ordering::Relaxed);
    usb_thread.join().expect("USB thread panicked");
    result?;

    Ok(())
}
