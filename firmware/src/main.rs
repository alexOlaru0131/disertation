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
