use crate::mission_controller::MissionType;
use crate::mission_controller::mission_process;
use crate::robot_controller::robot_process;
use std::error::Error;
use tokio::sync::mpsc;

pub mod cloud_communication;
pub mod legs;
pub mod mission_controller;
pub mod point;
pub mod robot_controller;

type AppError = Box<dyn Error + Send + Sync>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::result::Result<(), AppError> {
    let (mut tx_robot, mut rx_mission) = mpsc::channel::<MissionType>(8);
    let (mut tx_mission, mut rx_robot) = mpsc::channel::<MissionType>(8);

    let mp = tokio::spawn(mission_process(tx_robot, rx_robot));
    let rp = tokio::spawn(robot_process(tx_mission, rx_mission));
    mp.await??;
    rp.await??;

    Ok(())
}
