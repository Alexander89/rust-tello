use std::{
    convert::TryFrom,
    net::{SocketAddr, UdpSocket},
    string::FromUtf8Error,
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread::{self, sleep},
    time::{Duration, Instant},
};

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

impl CommandMode {
    fn create_state_receiver() -> Receiver<CommandModeState> {
        let (tx, state_receiver) = mpsc::channel::<CommandModeState>();
        thread::spawn(move || {
            let state_socket = UdpSocket::bind(&SocketAddr::from(([0, 0, 0, 0], 8890)))
                .expect("couldn't bind to command address");
            state_socket.set_nonblocking(true).unwrap();
            'udpReceiverLoop: loop {
                let mut buf = [0u8; 150];
                match state_socket.recv(&mut buf) {
                    Ok(_) => tx.send(CommandModeState::try_from(&buf).unwrap()).unwrap(),
                    Err(e) => {
                        if e.raw_os_error().unwrap_or(0) == 11 {
                            sleep(Duration::from_millis(500));
                        } else {
                            println!("BOOM: {:?}", e.to_string());
                            break 'udpReceiverLoop;
                        }
                    }
                }
            }
        });
        state_receiver
    }
    fn create_video_receiver(port: u16) -> Receiver<Vec<u8>> {
        let (video_sender, video_receiver) = mpsc::channel::<Vec<u8>>();
        thread::spawn(move || {
            let video_socket = UdpSocket::bind(&SocketAddr::from(([0, 0, 0, 0], port)))
                .expect("couldn't bind to command address");
            video_socket.set_nonblocking(true).unwrap();
            let mut res_buffer = [0u8; 20000];
            let mut ptr = 0;
            let mut buf = [0u8; 1460];
            loop {
                match video_socket.recv(&mut buf) {
                    Ok(size) => {
                        for v in 0..size {
                            res_buffer[ptr] = buf[v];
                            ptr += 1;
                        }
                        if size < 1460 {
                            println!("got frame: size {}", ptr);
                            video_sender.send(res_buffer[0..ptr].to_owned()).unwrap();
                            ptr = 0;
                            res_buffer = [0u8; 20000];
                        }
                    }
                    Err(_) => {
                        sleep(Duration::from_millis(100));
                    }
                }
            }
        });
        video_receiver
    }
}

impl From<UdpSocket> for CommandMode {
    fn from(socket: UdpSocket) -> CommandMode {
        // state receiver
        let state_receiver = Self::create_state_receiver();

        // video receiver
        let video_receiver = Self::create_video_receiver(11111);

        Self {
            socket: Arc::new(Mutex::new(socket)),
            odometry: Odometry::default(),
            state_receiver,
            video_receiver,
        }
    }
}

impl CommandMode {
    pub fn new(ip: &str) -> Self {
        let socket = UdpSocket::bind(&SocketAddr::from(([0, 0, 0, 0], 8889)))
            .expect("couldn't bind to command address");
        socket.connect(ip).expect("connect command socket failed");
        Self::from(socket)
    }
}

impl CommandMode {
    async fn send_command(&self, command: &[u8]) -> Result<(), String> {
        let recv_socket = self.socket.clone();
        let timeout = Instant::now();
        async move {
            let socket = recv_socket.lock().unwrap();
            {
                // clear socket if something is left in there
                let mut buf = [0u8; 4192];
                let _ignore = socket.recv(&mut buf);
            }
            socket
                .send(&command)
                .map_err(|e| format!("Failed to send command to drone: {:?}", e))?;

            let mut buf = [0u8; 64];
            loop {
                let res = socket.recv(&mut buf);
                match res {
                    Err(e) => {
                        // 11 = Resource temporarily unavailable
                        if let Some(11) = e.raw_os_error() {
                            if timeout.elapsed() > Duration::new(30, 0) {
                                break Err("timeout".to_string());
                            }
                            std::thread::sleep(Duration::from_millis(300));
                        } else {
                            break Err(format!("socket error {:?}", e));
                        }
                    }
                    Ok(bytes) => {
                        break String::from_utf8(buf[..bytes].to_vec())
                            .map_err(|_| format!("Failed to read data {:?}", buf))
                            .and_then(|res| {
                                if res.starts_with("ok") {
                                    println!(
                                        "got OK for {:?}!",
                                        String::from_utf8(command.to_vec()).unwrap()
                                    );
                                    Ok(())
                                } else if res.starts_with("error") {
                                    Err(res)
                                } else {
                                    Err("Unknown response".to_string())
                                }
                            })
                    }
                }
            }
        }
        .await
    }

    pub async fn enable(&self) -> Result<(), String> {
        // println!("enable");
        self.send_command(b"command").await
    }
    pub async fn emergency(&self) -> Result<(), String> {
        // println!("emergency");
        self.send_command(b"emergency").await
    }
    pub async fn take_off(&mut self) -> Result<(), String> {
        // println!("take off");
        let r = self.send_command(b"takeoff").await;
        self.odometry.up(100);
        r
    }
    pub async fn land(&self) -> Result<(), String> {
        // println!("land");
        self.send_command(b"land").await
    }
    pub async fn video_on(&self) -> Result<(), String> {
        // println!("video on");
        self.send_command(b"streamon").await
    }
    pub async fn video_off(&self) -> Result<(), String> {
        // println!("video off");
        self.send_command(b"streamoff").await
    }
    pub async fn up(&mut self, step: u32) -> Result<(), String> {
        // println!("up");
        let step_norm = step.min(500).max(20);
        let command = format!("up {}", step_norm);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.odometry.up(step_norm)))
    }
    pub async fn down(&mut self, step: u32) -> Result<(), String> {
        // println!("down");
        let step_norm = step.min(500).max(20);
        let command = format!("down {}", step_norm);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.odometry.down(step_norm)))
    }
    pub async fn left(&mut self, step: u32) -> Result<(), String> {
        // println!("left");
        let step_norm = step.min(500).max(20);
        let command = format!("left {}", step_norm);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.odometry.left(step_norm)))
    }
    pub async fn right(&mut self, step: u32) -> Result<(), String> {
        // println!("right");
        let step_norm = step.min(500).max(20);
        let command = format!("right {}", step_norm);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.odometry.right(step_norm)))
    }
    pub async fn forward(&mut self, step: u32) -> Result<(), String> {
        // println!("forward");
        let step_norm = step.min(500).max(20);
        let command = format!("forward {}", step_norm);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.odometry.forward(step_norm)))
    }
    pub async fn back(&mut self, step: u32) -> Result<(), String> {
        // println!("back");
        let step_norm = step.min(500).max(20);
        self.odometry.back(step_norm);
        let command = format!("back {}", step_norm);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.odometry.back(step_norm)))
    }
    pub async fn cw(&mut self, step: u32) -> Result<(), String> {
        // println!("cw");
        let command = format!("cw {}", step);
        let step_norm = step.min(3600).max(1);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.odometry.cw(step_norm)))
    }
    pub async fn ccw(&mut self, step: u32) -> Result<(), String> {
        // println!("ccw");
        let step_norm = step.min(3600).max(1);
        let command = format!("ccw {}", step);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.odometry.ccw(step_norm)))
    }
    pub async fn go_to(&mut self, x: i32, y: i32, z: i32, speed: u8) -> Result<(), String> {
        // println!("speed");
        let x_norm = (x == 0).then(|| 0).unwrap_or(x.min(500).max(20));
        let y_norm = (y == 0).then(|| 0).unwrap_or(y.min(500).max(20));
        let z_norm = (z == 0).then(|| 0).unwrap_or(z.min(500).max(20));
        let speed_norm = speed.min(100).max(10);
        let command = format!("go {} {} {} {}", x_norm, y_norm, z_norm, speed_norm);
        println!("{}", command);
        self.send_command(&command.into_bytes()).await
    }
    pub async fn curve(
        &mut self,
        x1: u32,
        y1: u32,
        z1: u32,
        x2: u32,
        y2: u32,
        z2: u32,
        speed: u8,
    ) -> Result<(), String> {
        println!("curve");
        let x1_norm = x1.min(500).max(20);
        let y1_norm = y1.min(500).max(20);
        let z1_norm = z1.min(500).max(20);
        let x2_norm = x2.min(500).max(20);
        let y2_norm = y2.min(500).max(20);
        let z2_norm = z2.min(500).max(20);
        let speed_norm = speed.min(100).max(10);
        let command = format!(
            "curve {} {} {} {} {} {} {}",
            x1_norm, y1_norm, z1_norm, x2_norm, y2_norm, z2_norm, speed_norm
        );
        self.send_command(&command.into_bytes()).await
    }
    pub async fn speed(&self, speed: u8) -> Result<(), String> {
        println!("speed");
        let normalized_speed = speed.min(100).max(10);
        let command = format!("speed {}", normalized_speed);
        self.send_command(&command.into_bytes()).await
    }
}
