use i2cdev::{core::I2CDevice, linux::LinuxI2CDevice};
use icm20948::Icm20948;
use tokio::{
    sync::mpsc,
    time::{Duration, interval},
};

pub mod icm20948;

#[derive(Debug, Clone, Copy)]
enum Command {
    ReadImu,
    Shutdown,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel::<Command>(32);

    let i2c_thread = tokio::task::spawn_blocking(move || {
        let mut imu = Icm20948::default();

        if let Err(error) = imu.init() {
            eprintln!("error at imu init {error:?}");
            return;
        }

        while let Some(command) = rx.blocking_recv() {
            match command {
                Command::ReadImu => match imu.read_measurements() {
                    Ok(measurements) => println!("{:?}", measurements.values),
                    Err(error) => eprintln!("error at imu read {error:?}"),
                },
                Command::Shutdown => break,
            }
        }
    });

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

    tokio::signal::ctrl_c().await?;
    let _ = tx.send(Command::Shutdown).await;

    i2c_thread.await?;
    imu_task.await?;

    Ok(())
}
