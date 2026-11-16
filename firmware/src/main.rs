use icm20948::Icm20948;
use tokio::{
    sync::mpsc,
    time::{Duration, interval},
};

pub mod icm20948;
pub mod pca9685;

#[derive(Debug, Clone, Copy)]
enum Command {
    ReadImu,
    Shutdown,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (tx, mut rx) = mpsc::channel::<Command>(32);

    let mut i2c_thread = tokio::task::spawn_blocking(move || {
        let mut imu = Icm20948::new()?;
        imu.init()?;

        while let Some(command) = rx.blocking_recv() {
            match command {
                Command::ReadImu => match imu.read_measurements() {
                    Ok(measurements) => println!("{:?}", measurements.values),
                    Err(error) => eprintln!("error at imu read {error:?}"),
                },
                Command::Shutdown => break,
            }
        }
        Ok::<(), icm20948::SensorError>(())
    });

    let imu_tx = tx.clone();
    let imu_task = tokio::spawn(async move {
        loop {
            if imu_tx.send(Command::ReadImu).await.is_err() {
                break;
            }
        }
    });

    let result = tokio::select! {
        result = &mut i2c_thread => result?,
        signal = tokio::signal::ctrl_c() => {
            signal?;
            imu_task.abort();
            let _ = tx.send(Command::Shutdown).await;
            i2c_thread.await?
        }
    };
    imu_task.abort();
    let _ = imu_task.await;
    result?;

    Ok(())
}
