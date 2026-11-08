use crate::cloud_communication::CloudCommunicator;
use crate::legs::Leg;
use crate::mission_controller::MissionType;
use crate::point::Point;
use futures_util::SinkExt;
use std::error::Error;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const L1: f64 = 0.1;
const L2: f64 = 0.1;
const L3: f64 = 0.05;

type AppError = Box<dyn Error + Send + Sync>;

pub struct RobotController {
    robot_position: Point,
    legs: [Leg; 6],
    cloud_communicator: CloudCommunicator,
}

impl RobotController {
    pub fn new(cloud_communicator: CloudCommunicator) -> Self {
        let robot_position = Point::new(0.0, 0.0, 0.05);
        let legs = [
            Leg::new(
                1,
                Point::new(L1 + L2, 0.0, -L3),
                0.083,
                Point::new(0.06685, -0.072, 0.0),
            ),
            Leg::new(
                2,
                Point::new(L1 + L2, 0.0, -L3),
                0.06685,
                Point::new(0.0755, 0.0, 0.0),
            ),
            Leg::new(
                3,
                Point::new(L1 + L2, 0.0, -L3),
                0.083,
                Point::new(0.06685, 0.072, 0.0),
            ),
            Leg::new(
                4,
                Point::new(-(L1 + L2), 0.0, -L3),
                0.083,
                Point::new(-0.06685, -0.072, 0.0),
            ),
            Leg::new(
                5,
                Point::new(-(L1 + L2), 0.0, -L3),
                0.06685,
                Point::new(-0.0755, 0.0, 0.0),
            ),
            Leg::new(
                6,
                Point::new(-(L1 + L2), 0.0, -L3),
                0.083,
                Point::new(-0.06685, 0.072, 0.0),
            ),
        ];
        Self {
            robot_position,
            legs,
            cloud_communicator,
        }
    }

    pub fn load_state(&mut self) -> Vec<f64> {
        vec![
            self.robot_position.x,
            self.robot_position.y,
            self.robot_position.z,
            self.robot_position.rot_x,
            self.robot_position.rot_y,
            self.robot_position.rot_z,
        ]
    }

    pub async fn walk(&mut self, target_point: Point) -> std::result::Result<(), AppError> {
        for k in 0..60 {
            for leg in &mut self.legs {
                if [1, 3, 5].contains(&(leg.id as i32)) {
                    leg.straight_gait_swing_kinematics(k);
                    leg.inverse_kinematics(leg.leg_position);
                }
            }

            let mut robot_state = self.robot_position.to_vec();
            for leg in &mut self.legs {
                robot_state.extend(leg.to_vec());
            }

            self.send_state(robot_state).await?;

            sleep(Duration::from_millis(100)).await;
        }

        for k in 0..60 {
            for leg in &mut self.legs {
                if [2, 4, 6].contains(&(leg.id as i32)) {
                    leg.straight_gait_swing_kinematics(k);
                    leg.inverse_kinematics(leg.leg_position);
                }
            }

            let mut robot_state = self.robot_position.to_vec();
            for leg in &mut self.legs {
                robot_state.extend(leg.to_vec());
            }

            self.send_state(robot_state).await?;

            sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    pub async fn send_state(&mut self, robot_state: Vec<f64>) -> std::result::Result<(), AppError> {
        let payload = serde_json::to_string(&robot_state)?;

        self.cloud_communicator
            .socket
            .send(Message::Text(format!("STATE {payload}").into()))
            .await?;

        Ok(())
    }
}

pub async fn robot_process(
    mut tx: Sender<MissionType>,
    mut rx: Receiver<MissionType>,
) -> std::result::Result<(), AppError> {
    let (socket, _) = connect_async("ws://127.0.0.1:8080").await?;

    let cloud_communicator = CloudCommunicator::new(socket);
    let mut robot_controller = RobotController::new(cloud_communicator);

    while let Some(mission) = rx.recv().await {
        match mission {
            MissionType::Move { target_position } => {
                robot_controller.walk(target_position).await?;
            }
        }
    }

    robot_controller.cloud_communicator.close().await?;
    Ok(())
}
