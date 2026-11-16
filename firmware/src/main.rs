use icm20948::Icm20948;
use pca9685::Pca9685;
use tokio::{
    sync::mpsc,
    time::{Duration, interval},
};

pub mod icm20948;
pub mod pca9685;

#[derive(Debug, Clone, Copy)]
enum Command {
    ReadImu,
    SetPwm([u16; 20]),
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
                    Command::ReadImu => match imu.read_measurements() {
                        Ok(measurements) => println!("{:?}", measurements.values),
                        Err(error) => eprintln!("error at imu read {error:?}"),
                    },
                    Command::SetPwm(angles) => {
                        if let Err(error) = pca42.set_pwm_vals(&angles[..10]) {
                            eprintln!("error at PCA 0x42: {error:?}");
                        }
                        if let Err(error) = pca41.set_pwm_vals(&angles[10..]) {
                            eprintln!("error at PCA 0x41: {error:?}");
                        }
                    }
                    Command::Shutdown => break,
                }
            }
            Ok(())
        },
    );

    let imu_tx = tx.clone();
    let imu_task = tokio::spawn(async move {
        let mut tick = interval(Duration::from_millis(500));
        loop {
            tick.tick().await;
            if imu_tx.send(Command::ReadImu).await.is_err() {
                break;
            }
        }
    });

    let pwm_tx = tx.clone();
    let pwm_task = tokio::spawn(async move {
        let mut tick = interval(Duration::from_millis(150));
        loop {
            for angle in (0..=9_000u16)
                .step_by(500)
                .chain((500..9_000u16).step_by(500).rev())
            {
                tick.tick().await;
                if pwm_tx.send(Command::SetPwm([angle; 20])).await.is_err() {
                    return;
                }
            }
        }
    });

    let result = tokio::select! {
        result = &mut i2c_thread => result?,
        signal = tokio::signal::ctrl_c() => {
            signal?;
            imu_task.abort();
            pwm_task.abort();
            let _ = tx.send(Command::Shutdown).await;
            i2c_thread.await?
        }
    };
    imu_task.abort();
    pwm_task.abort();
    let _ = imu_task.await;
    let _ = pwm_task.await;
    result?;

    Ok(())
}
