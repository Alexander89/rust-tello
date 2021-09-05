use std::{
    convert::TryFrom,
    net::SocketAddr,
    string::FromUtf8Error,
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread::{self, sleep},
    time::{Duration, Instant},
};

#[cfg(not(feature = "tokio_async"))]
use std::net::UdpSocket;
#[cfg(feature = "tokio_async")]
use tokio::net::UdpSocket;

use crate::odometry::Odometry;
pub struct CommandMode {
    socket: Arc<Mutex<UdpSocket>>,
    pub odometry: Odometry,
    pub state_receiver: Receiver<CommandModeState>,
    pub video_receiver: Receiver<Vec<u8>>,
}
#[derive(Default, Debug)]
pub struct CommandModeState {
    pub pitch: i16, // 0
    pub roll: i16,  // 0
    pub yaw: i16,   // -45
    pub vgx: i16,   // 0
    pub vgy: i16,   // 0
    pub vgz: i16,   // 0
    pub templ: i8,  // 69
    pub temph: i8,  // 70
    pub tof: i16,   // 10
    pub h: i16,     // 0
    pub bat: i8,    // 92
    pub baro: f32,  // 548.55
    pub time: f32,  // 0
    pub agx: f32,   // -5.00
    pub agy: f32,   // 0.00
    pub agz: f32,   // -998.00
}

impl TryFrom<&[u8; 150]> for CommandModeState {
    type Error = FromUtf8Error;
    fn try_from(error: &[u8; 150]) -> Result<Self, FromUtf8Error> {
        String::from_utf8(error.to_vec()).and_then(|str| {
            Ok(str
                .split(';')
                .fold(CommandModeState::default(), |mut acc, v| {
                    let param: Vec<&str> = v.split(':').collect();
                    match (param.get(0).and_then(|v| Some(v.clone())), param.get(1)) {
                        (Some("pitch"), Some(value)) => acc.pitch = value.parse().unwrap(),
                        (Some("roll"), Some(value)) => acc.roll = value.parse().unwrap(),
                        (Some("yaw"), Some(value)) => acc.yaw = value.parse().unwrap(),
                        (Some("vgx"), Some(value)) => acc.vgx = value.parse().unwrap(),
                        (Some("vgy"), Some(value)) => acc.vgy = value.parse().unwrap(),
                        (Some("vgz"), Some(value)) => acc.vgz = value.parse().unwrap(),
                        (Some("templ"), Some(value)) => acc.templ = value.parse().unwrap(),
                        (Some("temph"), Some(value)) => acc.temph = value.parse().unwrap(),
                        (Some("tof"), Some(value)) => acc.tof = value.parse().unwrap(),
                        (Some("h"), Some(value)) => acc.h = value.parse().unwrap(),
                        (Some("bat"), Some(value)) => acc.bat = value.parse().unwrap(),
                        (Some("baro"), Some(value)) => acc.baro = value.parse().unwrap(),
                        (Some("time"), Some(value)) => acc.time = value.parse().unwrap(),
                        (Some("agx"), Some(value)) => acc.agx = value.parse().unwrap(),
                        (Some("agy"), Some(value)) => acc.agy = value.parse().unwrap(),
                        (Some("agz"), Some(value)) => acc.agz = value.parse().unwrap(),
                        _ => (),
                    }
                    acc
                }))
        })
    }
}
