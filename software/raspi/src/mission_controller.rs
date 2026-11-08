use crate::point::Point;
use std::error::Error;
use tokio::sync::mpsc::{Receiver, Sender};

type AppError = Box<dyn Error + Send + Sync>;

pub enum MissionType {
    Move { target_position: Point },
}

impl MissionType {}

pub async fn mission_process(
    tx: Sender<MissionType>,
    rx: Receiver<MissionType>,
) -> std::result::Result<(), AppError> {
    let new_position = Point::new(0.0, 0.0, 0.0);
    tx.send(MissionType::Move {
        target_position: new_position,
    })
    .await?;

    Ok(())
}
