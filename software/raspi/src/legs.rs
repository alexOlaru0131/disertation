use crate::point::Point;
use libm::{asin, atan2, cos, sin};
use std::f64::consts::PI;

const L1: f64 = 0.1;
const L2: f64 = 0.1;
const L3: f64 = 0.05;
const L_OF: f64 = 0.02;
const D: f64 = 0.12;
const N: f64 = 60.0;
const LIFT: f64 = 0.05;
const LONG: f64 = 0.12;

#[derive(Clone, Copy)]
pub struct Leg {
    pub id: u8,
    pub leg_position: Point,
    leg_motor_angles: [f64; 3],
    l_co: f64,
    point_om: Point,
}

impl Leg {
    pub fn new(id: u8, initial_point: Point, l_co: f64, point_om: Point) -> Self {
        let mut leg = Self {
            id,
            leg_position: initial_point,
            leg_motor_angles: [0.0; 3],
            l_co,
            point_om,
        };

        leg.inverse_kinematics(initial_point);

        leg
    }

    pub fn straight_gait_swing_kinematics(&mut self, k: u8) -> Point {
        let x = self.point_om.x + L_OF * cos(self.leg_position.rot_z);
        let y = self.point_om.y
            + L_OF * sin(self.leg_position.rot_z)
            + D * (k as f64 / N - (1 / 2) as f64);
        let z = self.point_om.z - 0.15
            + LIFT * cos(PI * (y - self.point_om.y - L_OF * sin(self.leg_position.rot_z)) / LONG);

        self.leg_position = Point::new(x, y, z);

        Point {
            x,
            y,
            z,
            rot_x: 0.0,
            rot_y: 0.0,
            rot_z: 0.0,
        }
    }

    pub fn straight_gait_stance_kinematics(&mut self, k: u8) -> Point {
        let x = self.point_om.x + L_OF * cos(self.leg_position.rot_z);
        let y = self.point_om.y
            + L_OF * sin(self.leg_position.rot_z)
            + D * (-(k as f64) / N + (1 / 2) as f64);
        let z = self.point_om.z - 0.15;

        self.leg_position = Point::new(x, y, z);

        Point {
            x,
            y,
            z,
            rot_x: 0.0,
            rot_y: 0.0,
            rot_z: 0.0,
        }
    }

    pub fn swivel_gait_swing_kinematics(&mut self, k: u8) -> Point {
        let x = cos(self.leg_position.rot_z) * (self.l_co + L_OF)
            - sin(self.leg_position.rot_z) * D * (k as f64 / N - (1 / 2) as f64);
        let y = sin(self.leg_position.rot_z) * (self.l_co + L_OF)
            + cos(self.leg_position.rot_z) * D * (k as f64 / N - (1 / 2) as f64);
        let z = -0.15 + LIFT * cos(PI * (k as f64 / N - (1 / 2) as f64));

        self.leg_position = Point::new(x, y, z);

        Point {
            x,
            y,
            z,
            rot_x: 0.0,
            rot_y: 0.0,
            rot_z: 0.0,
        }
    }

    pub fn swivel_gait_stance_kinematics(&mut self, k: u8) -> Point {
        let x = cos(self.leg_position.rot_z) * (self.l_co + L_OF)
            - sin(self.leg_position.rot_z) * D * (k as f64 / N - (1 / 2) as f64);
        let y = sin(self.leg_position.rot_z) * (self.l_co + L_OF)
            + cos(self.leg_position.rot_z) * D * (k as f64 / N - (1 / 2) as f64);
        let z = -0.15;

        self.leg_position = Point::new(x, y, z);

        Point {
            x,
            y,
            z,
            rot_x: 0.0,
            rot_y: 0.0,
            rot_z: 0.0,
        }
    }

    pub fn inverse_kinematics(&mut self, target_point: Point) -> [f64; 3] {
        let Point {
            x,
            y,
            z,
            rot_x: _,
            rot_y: _,
            rot_z: _,
        } = target_point;

        let theta1 = atan2(y, x);
        let c1 = cos(theta1);
        let s1 = sin(theta1);

        let theta3 = asin(
            (f64::powi(c1 * x + s1 * y - L1, 2) + f64::powi(z, 2)
                - f64::powi(L3, 2)
                - f64::powi(L2, 2))
                / (2.0 * L2 * L3),
        ) - PI / 2.0;
        let c3 = cos(theta3);
        let s3 = sin(theta3);

        let theta2 = asin(z / f64::sqrt(f64::powi(L3 * c3 + L2, 2) + f64::powi(L3 * s3, 2)))
            - atan2(L3 * s3, L3 * c3 + L2);

        self.leg_motor_angles = [
            theta1.to_degrees(),
            theta2.to_degrees(),
            theta3.to_degrees(),
        ];

        [
            theta1.to_degrees(),
            theta2.to_degrees(),
            theta3.to_degrees(),
        ]
    }

    pub fn to_vec(&mut self) -> Vec<f64> {
        let mut result_vec = vec![
            self.leg_position.x,
            self.leg_position.y,
            self.leg_position.z,
        ];

        result_vec.extend(self.leg_motor_angles);
        result_vec.extend(self.point_om.to_vec());
        result_vec.append(&mut vec![self.l_co]);

        result_vec
    }
}
