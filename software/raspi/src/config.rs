// Parametrii fizici ai robotului (metri). Raman ficsi in procesul Raspi.
pub const L1: f64 = 0.02;
pub const L2: f64 = 0.084;
pub const L3: f64 = 0.140;

// Proiectia orizontala a varfului piciorului fata de baza lui, nu lungimea coxa.
pub const L_OF: f64 = 0.10;
pub const D: f64 = 0.12;
pub const N: u8 = 60;
pub const LIFT: f64 = 0.05;
pub const LONG: f64 = 0.12;
pub const STANCE_HEIGHT: f64 = 0.15;
pub const BODY_HEIGHT: f64 = 0.15;
pub const SAMPLE_MS: u64 = 100;

pub const LEG_MOUNTS: [[f64; 3]; 6] = [
    [0.06685, -0.072, 0.0],
    [0.0755, 0.0, 0.0],
    [0.06685, 0.072, 0.0],
    [-0.06685, -0.072, 0.0],
    [-0.0755, 0.0, 0.0],
    [-0.06685, 0.072, 0.0],
];
pub const L_CO: [f64; 6] = [0.083, 0.06685, 0.083, 0.083, 0.06685, 0.083];
