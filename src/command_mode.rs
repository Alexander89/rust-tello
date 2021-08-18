use std::{
    convert::TryFrom,
    net::{SocketAddr, UdpSocket},
    string::FromUtf8Error,
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread::{self, sleep},
    time::Duration,
};

pub struct CommandMode {
    socket: Arc<Mutex<UdpSocket>>,
    pub position: Position,
    pub state_receiver: Receiver<CommandModeState>,
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

impl From<UdpSocket> for CommandMode {
    fn from(socket: UdpSocket) -> CommandMode {
        let (tx, rx) = mpsc::channel::<CommandModeState>();
        thread::spawn(move || {
            let video_socket = UdpSocket::bind(&SocketAddr::from(([0, 0, 0, 0], 11111)))
                .expect("couldn't bind to command address");
            video_socket.set_nonblocking(true).unwrap();
            let mut resBuffer = [0u8; 20000];
            let mut ptr = 0;
            let mut buf = [0u8; 1460];
            loop {
                match video_socket.recv(&mut buf) {
                    Ok(size) => {
                        if ptr == 0 {
                            for v in 0..size {
                                resBuffer[ptr] = buf[v];
                                ptr += 1;
                            }
                        } else {
                            for v in 0..size {
                                resBuffer[ptr] = buf[v];
                                ptr += 1;
                            }
                        }
                        println!(
                            "frame {} {}{}{}{}{}{}{}{}{}",
                            size,
                            buf[0],
                            buf[1],
                            buf[2],
                            buf[3],
                            buf[4],
                            resBuffer[0],
                            resBuffer[1],
                            resBuffer[2],
                            resBuffer[3]
                        );

                        if size < 1460 {
                            println!("size is: {}", ptr);
                            ptr = 0;
                            resBuffer = [0u8; 20000];
                            video_socket
                                .send_to(&resBuffer[0..ptr], "127.0.0.1:11001")
                                .unwrap();
                        }
                    }
                    Err(_) => {
                        sleep(Duration::from_millis(100));
                    }
                }
            }
        });
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
                            println!("Hmmm, what do i do here {:?}", e.to_string());
                            break 'udpReceiverLoop;
                        }
                    }
                }
            }
        });

        Self {
            socket: Arc::new(Mutex::new(socket)),
            position: Position::default(),
            state_receiver: rx,
        }
    }
}

impl CommandMode {
    pub fn new(ip: &str) -> Self {
        let socket = UdpSocket::bind(&SocketAddr::from(([0, 0, 0, 0], 8889)))
            .expect("couldn't bind to command address");
        socket.set_nonblocking(true).unwrap();
        socket.connect(ip).expect("connect command socket failed");
        Self::from(socket)
    }
}

impl CommandMode {
    async fn send_command(&self, command: &[u8]) -> Result<(), String> {
        self.socket
            .lock()
            .map_err(|_| "Failed to lock the socket".to_string())
            .and_then(|socket| {
                socket
                    .send(&command)
                    .map_err(|_| "Failed to send command to drone".to_string())
            })?;
        let recv_socket = self.socket.clone();

        let drone_reply = async move {
            let mut buf = [0u8; 8];
            let res: Result<(), String> = recv_socket
                .lock()
                .and_then(|s| Ok(s.recv(&mut buf)))
                .map_err(|_| "no data received".to_string())
                .and_then(move |_| -> Result<(), String> {
                    String::from_utf8(buf.to_vec())
                        .map_err(|_| format!("Failed to read data {:?}", buf))
                        .and_then(|res| {
                            if res.starts_with("ok") {
                                Ok(())
                            } else if res.starts_with(0 as char) {
                                Err("did not reply".to_string())
                            } else {
                                Err("Drone replies Error".to_string())
                            }
                        })
                });
            res
        };
        drone_reply.await
    }

    pub async fn enable(&self) -> Result<(), String> {
        println!("enable");
        self.send_command(b"command").await
    }
    pub async fn emergency(&self) -> Result<(), String> {
        println!("emergency");
        self.send_command(b"emergency").await
    }
    pub async fn take_off(&self) -> Result<(), String> {
        println!("take off");
        self.send_command(b"takeoff").await
    }
    pub async fn land(&self) -> Result<(), String> {
        println!("land");
        self.send_command(b"land").await
    }
    pub async fn video_on(&self) -> Result<(), String> {
        println!("video on");
        self.send_command(b"streamon").await
    }
    pub async fn video_off(&self) -> Result<(), String> {
        println!("video off");
        self.send_command(b"streamoff").await
    }
    pub async fn up(&mut self, step: u32) -> Result<(), String> {
        println!("up");
        let step_norm = step.min(500).max(20);
        let command = format!("up {}", step_norm);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.position.up(step_norm)))
    }
    pub async fn down(&mut self, step: u32) -> Result<(), String> {
        println!("down");
        let step_norm = step.min(500).max(20);
        let command = format!("down {}", step_norm);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.position.down(step_norm)))
    }
    pub async fn left(&mut self, step: u32) -> Result<(), String> {
        println!("left");
        let step_norm = step.min(500).max(20);
        let command = format!("left {}", step_norm);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.position.left(step_norm)))
    }
    pub async fn right(&mut self, step: u32) -> Result<(), String> {
        println!("right");
        let step_norm = step.min(500).max(20);
        let command = format!("right {}", step_norm);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.position.right(step_norm)))
    }
    pub async fn forward(&mut self, step: u32) -> Result<(), String> {
        println!("forward");
        let step_norm = step.min(500).max(20);
        let command = format!("forward {}", step_norm);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.position.forward(step_norm)))
    }
    pub async fn back(&mut self, step: u32) -> Result<(), String> {
        println!("back");
        let step_norm = step.min(500).max(20);
        self.position.back(step_norm);
        let command = format!("back {}", step_norm);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.position.back(step_norm)))
    }
    pub async fn cw(&mut self, step: u32) -> Result<(), String> {
        println!("cw");
        let command = format!("cw {}", step);
        let step_norm = step.min(3600).max(1);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.position.cw(step_norm)))
    }
    pub async fn ccw(&mut self, step: u32) -> Result<(), String> {
        println!("ccw");
        let step_norm = step.min(3600).max(1);
        let command = format!("ccw {}", step);
        self.send_command(&command.into_bytes())
            .await
            .and_then(|_| Ok(self.position.ccw(step_norm)))
    }
    pub async fn go_to(&mut self, x: u32, y: u32, z: u32, speed: u8) -> Result<(), String> {
        println!("speed");
        let x_norm = x.min(500).max(20);
        let y_norm = y.min(500).max(20);
        let z_norm = z.min(500).max(20);
        let speed_norm = speed.min(100).max(10);
        let command = format!("go {} {} {} {}", x_norm, y_norm, z_norm, speed_norm);
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

#[derive(Default, Debug)]
pub struct Position {
    x: i64,
    y: i64,
    z: i64,
    rot: i64,
}

impl Position {
    fn up(&mut self, z: u32) -> () {
        self.z += z.max(20).min(500) as i64;
    }
    fn down(&mut self, z: u32) -> () {
        self.z -= z.max(20).min(500) as i64;
    }
    fn right(&mut self, x: u32) -> () {
        self.x += x.max(20).min(500) as i64;
    }
    fn left(&mut self, x: u32) -> () {
        self.x -= x.max(20).min(500) as i64;
    }
    fn forward(&mut self, y: u32) -> () {
        self.y += y.max(20).min(500) as i64;
    }
    fn back(&mut self, y: u32) -> () {
        self.y -= y.max(20).min(500) as i64;
    }
    fn cw(&mut self, rot: u32) -> () {
        self.rot += rot.max(1).min(3600) as i64;
    }
    fn ccw(&mut self, rot: u32) -> () {
        self.rot -= rot.max(1).min(3600) as i64;
    }
}
