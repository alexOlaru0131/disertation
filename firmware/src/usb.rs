use crate::Command;
use serde::{Deserialize, Serialize};
use serialport::SerialPort;
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};

const MAX_LINE: usize = 4096;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request {
    ReadImu { id: u64 },
    SetPwm { id: u64, angles: Vec<u16> },
    ReadStops { id: u64 },
    TurnOnLaserLed { id: u64 },
    TurnOffLaserLed { id: u64 },
    TurnOnLed { id: u64 },
    TurnOffLed { id: u64 },
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response {
    Imu { id: u64, values: [i32; 10] },
    Stops { id: u64, values: [u8; 6] },
    Ack { id: u64 },
    Error { id: u64, message: String },
}

impl Response {
    fn error(id: u64, message: impl Into<String>) -> Self {
        Self::Error {
            id,
            message: message.into(),
        }
    }
}

fn send_ack_command(
    id: u64,
    tx: &mpsc::Sender<Command>,
    make_command: impl FnOnce(oneshot::Sender<Result<(), String>>) -> Command,
) -> Response {
    let (reply, answer) = oneshot::channel();
    if tx.blocking_send(make_command(reply)).is_err() {
        return Response::error(id, "workerul hardware este oprit");
    }
    match answer.blocking_recv() {
        Ok(Ok(())) => Response::Ack { id },
        Ok(Err(error)) => Response::error(id, error),
        Err(_) => Response::error(id, "workerul hardware nu a raspuns"),
    }
}

fn read_stops(id: u64, tx: &mpsc::Sender<Command>) -> Response {
    let (reply, answer) = oneshot::channel();
    if tx.blocking_send(Command::ReadStops(reply)).is_err() {
        return Response::error(id, "workerul GPIO este oprit");
    }
    match answer.blocking_recv() {
        Ok(Ok(values)) => Response::Stops { id, values },
        Ok(Err(error)) => Response::error(id, error),
        Err(_) => Response::error(id, "workerul GPIO nu a raspuns"),
    }
}

fn handle_request(line: &[u8], tx: &mpsc::Sender<Command>) -> Response {
    let request: Request = match serde_json::from_slice(line) {
        Ok(request) => request,
        Err(error) => return Response::error(0, format!("comanda invalida: {error}")),
    };

    match request {
        Request::ReadImu { id } => {
            let (reply, answer) = oneshot::channel();
            if tx.blocking_send(Command::ReadImu(reply)).is_err() {
                return Response::error(id, "workerul I2C este oprit");
            }
            match answer.blocking_recv() {
                Ok(Ok(values)) => Response::Imu { id, values },
                Ok(Err(error)) => Response::error(id, error),
                Err(_) => Response::error(id, "workerul I2C nu a raspuns"),
            }
        }
        Request::SetPwm { id, angles } => {
            if angles.len() != 20 || angles.iter().any(|&angle| angle > 36_000) {
                return Response::error(id, "sunt necesare 20 de unghiuri intre 0 si 36000");
            }
            let mut pwm = [0u16; 20];
            pwm.copy_from_slice(&angles);
            let (reply, answer) = oneshot::channel();
            if tx.blocking_send(Command::SetPwm(pwm, reply)).is_err() {
                return Response::error(id, "workerul I2C este oprit");
            }
            match answer.blocking_recv() {
                Ok(Ok(())) => Response::Ack { id },
                Ok(Err(error)) => Response::error(id, error),
                Err(_) => Response::error(id, "workerul I2C nu a raspuns"),
            }
        }
        Request::ReadStops { id } => read_stops(id, tx),
        Request::TurnOnLaserLed { id } => send_ack_command(id, tx, Command::TurnOnLaserLed),
        Request::TurnOffLaserLed { id } => send_ack_command(id, tx, Command::TurnOffLaserLed),
        Request::TurnOnLed { id } => send_ack_command(id, tx, Command::TurnOnLed),
        Request::TurnOffLed { id } => send_ack_command(id, tx, Command::TurnOffLed),
    }
}

fn send_response(port: &mut dyn SerialPort, response: &Response) -> io::Result<()> {
    let mut message = serde_json::to_vec(response).map_err(io::Error::other)?;
    message.push(b'\n');
    port.write_all(&message)?;
    port.flush()
}

fn read_commands(
    port: &mut dyn SerialPort,
    tx: &mpsc::Sender<Command>,
    stop: &AtomicBool,
) -> io::Result<()> {
    let mut buffer = [0u8; 256];
    let mut line = Vec::new();
    let mut oversized = false;

    while !stop.load(Ordering::Relaxed) {
        match port.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(count) => {
                for &byte in &buffer[..count] {
                    if byte == b'\n' {
                        if oversized {
                            send_response(port, &Response::error(0, "comanda prea lunga"))?;
                        } else if !line.is_empty() {
                            send_response(port, &handle_request(&line, tx))?;
                        }
                        line.clear();
                        oversized = false;
                    } else if !oversized {
                        if line.len() == MAX_LINE {
                            oversized = true;
                        } else {
                            line.push(byte);
                        }
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

pub fn serve(tx: mpsc::Sender<Command>, stop: Arc<AtomicBool>) {
    let path = std::env::var("ROBOT_USB_PORT").unwrap_or_else(|_| "/dev/ttyGS0".into());

    while !stop.load(Ordering::Relaxed) {
        match serialport::new(&path, 115_200)
            .timeout(Duration::from_millis(250))
            .open()
        {
            Ok(mut port) => {
                if let Err(error) = read_commands(port.as_mut(), &tx, &stop) {
                    eprintln!("USB {path}: {error}");
                }
            }
            Err(error) => eprintln!("USB {path}: {error}"),
        }
        if !stop.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_secs(1));
        }
    }
}
