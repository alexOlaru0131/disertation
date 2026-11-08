#[derive(Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub rot_x: f64,
    pub rot_y: f64,
    pub rot_z: f64,
}

impl Point {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            x,
            y,
            z,
            rot_x: 0.0,
            rot_y: 0.0,
            rot_z: 0.0,
        }
    }

    pub fn update(&self, x: f64, y: f64, z: f64) -> Self {
        Self {
            x,
            y,
            z,
            rot_x: self.rot_x,
            rot_y: self.rot_y,
            rot_z: self.rot_z,
        }
    }

    pub fn to_vec(&mut self) -> Vec<f64> {
        vec![self.x, self.y, self.z, self.rot_x, self.rot_y, self.rot_z]
    }
}
