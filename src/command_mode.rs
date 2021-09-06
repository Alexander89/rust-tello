use std::{convert::TryFrom, net::SocketAddr, string::FromUtf8Error, time::Duration};

#[cfg(not(feature = "tokio_async"))]
use std::net::UdpSocket;

#[cfg(not(feature = "tokio_async"))]
use std::time::Instant;
#[cfg(feature = "tokio_async")]
use tokio::net::UdpSocket;
#[cfg(feature = "tokio_async")]
use tokio::time::{sleep, timeout};

#[cfg(not(feature = "tokio_async"))]
use std::sync::mpsc;
#[cfg(feature = "tokio_async")]
use tokio::sync::{mpsc, watch};

#[cfg(not(feature = "tokio_async"))]
type StateReceiver<T> = mpsc::Receiver<T>;
#[cfg(feature = "tokio_async")]
type StateReceiver<T> = watch::Receiver<Option<T>>;

use crate::odometry::Odometry;

/// Command mode for your tello drone. to leave the command mode, you have to reboot the drone.
///
/// The CommandMode provides following information to you:
///
/// -   `state_receiver(): Option<Receiver<CommandModeState>>`: parsed incoming state packages from the drone. You will take the ownership, you could do this only once.
/// -   `video_receiver(): Option<Receiver<Vec<u8>>>`: Video frames (h264) from the drone. You will take the ownership, you could do this only once.
/// -   `odometry: Odometry` odometer data for your movements.
#[derive(Debug)]
pub struct CommandMode {
    peer_addr: SocketAddr,
    state_receiver: Option<StateReceiver<CommandModeState>>,
    video_receiver: Option<mpsc::Receiver<Vec<u8>>>,
    pub odometry: Odometry,
}
#[derive(Default, Debug, Clone)]
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
    pub bat: u8,    // 92
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

#[cfg(not(feature = "tokio_async"))]
impl CommandMode {
    fn create_state_receiver() -> mpsc::Receiver<CommandModeState> {
        let (tx, state_receiver) = mpsc::channel::<CommandModeState>();
        std::thread::spawn(move || {
            let state_socket = UdpSocket::bind(&SocketAddr::from(([0, 0, 0, 0], 8890)))
                .expect("couldn't bind to command address");
            state_socket.set_nonblocking(true).unwrap();
            'udpReceiverLoop: loop {
                let mut buf = [0u8; 150];
                match state_socket.recv(&mut buf) {
                    Ok(_) => {
                        if let Ok(state) = CommandModeState::try_from(&buf) {
                            tx.send(state).unwrap()
                        }
                    }
                    Err(e) => {
                        if e.raw_os_error().unwrap_or(0) == 11 {
                            std::thread::sleep(Duration::from_millis(500));
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

    fn create_video_receiver(port: u16) -> mpsc::Receiver<Vec<u8>> {
        let (video_sender, video_receiver) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
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
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        });
        video_receiver
    }
}
#[cfg(feature = "tokio_async")]
impl CommandMode {
    fn create_state_receiver() -> StateReceiver<CommandModeState> {
        let (tx, state_receiver) = watch::channel::<Option<CommandModeState>>(None);
        tokio::spawn(async move {
            let state_socket = UdpSocket::bind(&SocketAddr::from(([0, 0, 0, 0], 8890)))
                .await
                .expect("couldn't bind to command address");

            let mut buf = [0u8; 150];
            while let Ok((len, addr)) = state_socket.recv_from(&mut buf).await {
                println!("{:?} bytes received from {:?}", len, addr);
                if let Ok(data) = CommandModeState::try_from(&buf) {
                    let _ = tx.send(Some(data));
                }
            }
        });
        state_receiver
    }

    fn create_video_receiver(port: u16) -> mpsc::Receiver<Vec<u8>> {
        let (video_sender, video_receiver) = mpsc::channel::<Vec<u8>>(50);
        tokio::spawn(async move {
            let video_socket = UdpSocket::bind(&SocketAddr::from(([0, 0, 0, 0], port)))
                .await
                .expect("couldn't bind to command address");

            let mut res_buffer = [0u8; 20000];
            let mut ptr = 0;
            let mut buf = [0u8; 1460];
            loop {
                while let Ok((size, _)) = video_socket.recv_from(&mut buf).await {
                    for v in 0..size {
                        res_buffer[ptr] = buf[v];
                        ptr += 1;
                    }
                    if size < 1460 {
                        println!("got frame: size {}", ptr);
                        let _ = video_sender.send(res_buffer[0..ptr].to_owned());
                        ptr = 0;
                        res_buffer = [0u8; 20000];
                    }
                }
            }
        });
        video_receiver
    }
}

impl From<SocketAddr> for CommandMode {
    /// Constructs a new CommandMode from a SocketAddr.
    ///
    /// The state and the video frames receivers are spawned and provide those information
    /// if the drone already sends them. Otherwise you have to `enable()` the drone fist.
    fn from(peer_addr: SocketAddr) -> CommandMode {
        Self {
            peer_addr,
            odometry: Odometry::default(),
            state_receiver: Some(Self::create_state_receiver()),
            video_receiver: Some(Self::create_video_receiver(11111)),
        }
    }
}

impl CommandMode {
    /// Constructs a new CommandMode from a ip address `<ip>:<port>`.
    ///
    /// The state and the video frames receivers are spawned and provide those information
    /// if the drone already sends them.  Otherwise you have to `enable()` the drone fist.
    pub async fn new(ip: &str) -> Result<Self, std::io::Error> {
        Ok(Self::from(ip.parse::<SocketAddr>().unwrap()))
    }
    /// Take over the ownership of the state receiver. This method returns once the receiver and
    /// returns `None` afterwards
    ///
    /// If you using `tokio_async` you will always get the last known value. otherwise, you will
    /// get a channel of the incoming data.
    pub fn state_receiver(&mut self) -> Option<StateReceiver<CommandModeState>> {
        let mut recv = None;
        std::mem::swap(&mut recv, &mut self.state_receiver);
        recv
    }

    /// Take over the ownership of the video receiver. This method returns once the receiver and
    /// returns `None` afterwards
    pub fn video_receiver(&mut self) -> Option<mpsc::Receiver<Vec<u8>>> {
        let mut recv = None;
        std::mem::swap(&mut recv, &mut self.video_receiver);
        recv
    }
}

#[cfg(feature = "tokio_async")]
impl CommandMode {
    async fn send_command(&self, command: Vec<u8>) -> Result<(), String> {
        let peer = self.peer_addr.clone();
        let l = tokio::spawn(async move {
            let socket = UdpSocket::bind("0.0.0.0:8889")
                .await
                .map_err(|e| format!("can't create socket: {:?}", e))?;

            socket
                .send_to(&command, peer)
                .await
                .map_err(|e| format!("Failed to send command to drone: {:?}", e))?;

            let mut buf = [0u8; 64];
            let res = timeout(Duration::new(30, 0), socket.recv(&mut buf)).await;

            match res {
                Err(_) => Err(format!("timeout")),
                Ok(Err(e)) => {
                    // 11 = Resource temporarily unavailable
                    if let Some(11) = e.raw_os_error() {
                        sleep(Duration::from_millis(300)).await;
                        println!("I should restart the thing !?");
                        Err(format!("retry?"))
                    } else {
                        return Err(format!("socket error {:?}", e));
                    }
                }
                Ok(Ok(bytes)) => {
                    println!("got data {}, {:?}", bytes, buf[..bytes].to_vec());
                    return String::from_utf8(buf[..bytes].to_vec())
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
                        });
                }
            }
        });
        l.await.unwrap()
    }
}

#[cfg(not(feature = "tokio_async"))]
impl CommandMode {
    async fn send_command(&self, command: Vec<u8>) -> Result<(), String> {
        let timeout = Instant::now();
        async move {
            let socket = UdpSocket::bind("0.0.0.0:8889")
                .map_err(|e| format!("can't create socket: {:?}", e))?;
            socket
                .set_nonblocking(true)
                .map_err(|e| format!("set to non-Blocking failed: {:?}", e))?;
            {
                // clear socket if something is left in there
                let mut buf = [0u8; 4192];
                let _ignore = socket.recv(&mut buf);
            }
            socket
                .send_to(&command, self.peer_addr)
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
}

impl CommandMode {
    /// enables the drone. This command should be the first one you send.
    ///
    /// Note: There is no disable(). you have to power-cycle the drone to get it
    /// back to the normal mode.
    pub async fn enable(&self) -> Result<(), String> {
        self.send_command("command".into()).await
    }
    /// Emergency will stop the motors immediately without landing
    pub async fn emergency(&self) -> Result<(), String> {
        self.send_command("emergency".into()).await
    }
    /// starts the drone to 1 meter above the ground
    pub async fn take_off(&mut self) -> Result<(), String> {
        let r = self.send_command("takeoff".into()).await;
        self.odometry.up(100);
        r
    }
    /// Land the drone
    pub async fn land(&self) -> Result<(), String> {
        self.send_command("land".into()).await
    }
    /// Enable the drone to send video frames to the 11111 port of the command sender IP
    pub async fn video_on(&self) -> Result<(), String> {
        self.send_command("streamon".into()).await
    }
    /// Disable the video stream
    pub async fn video_off(&self) -> Result<(), String> {
        self.send_command("streamoff".into()).await
    }
    /// move upwards for 20-500 cm
    pub async fn up(&mut self, step: u32) -> Result<(), String> {
        let step_norm = step.min(500).max(20);
        let command = format!("up {}", step_norm);
        self.send_command(command.into())
            .await
            .and_then(|_| Ok(self.odometry.up(step_norm)))
    }
    /// move downwards for 20-500 cm (if possible)
    pub async fn down(&mut self, step: u32) -> Result<(), String> {
        let step_norm = step.min(500).max(20);
        let command = format!("down {}", step_norm);
        self.send_command(command.into())
            .await
            .and_then(|_| Ok(self.odometry.down(step_norm)))
    }
    /// move to the left for 20-500 cm
    pub async fn left(&mut self, step: u32) -> Result<(), String> {
        let step_norm = step.min(500).max(20);
        let command = format!("left {}", step_norm);
        self.send_command(command.into())
            .await
            .and_then(|_| Ok(self.odometry.left(step_norm)))
    }
    /// move to the right for 20-500 cm
    pub async fn right(&mut self, step: u32) -> Result<(), String> {
        let step_norm = step.min(500).max(20);
        let command = format!("right {}", step_norm);
        self.send_command(command.into())
            .await
            .and_then(|_| Ok(self.odometry.right(step_norm)))
    }
    /// move forwards for 20-200 cm
    pub async fn forward(&mut self, step: u32) -> Result<(), String> {
        let step_norm = step.min(500).max(20);
        let command = format!("forward {}", step_norm);
        self.send_command(command.into())
            .await
            .and_then(|_| Ok(self.odometry.forward(step_norm)))
    }
    /// move backwards for 20 - 500 cm
    pub async fn back(&mut self, step: u32) -> Result<(), String> {
        let step_norm = step.min(500).max(20);
        self.odometry.back(step_norm);
        let command = format!("back {}", step_norm);
        self.send_command(command.into())
            .await
            .and_then(|_| Ok(self.odometry.back(step_norm)))
    }
    /// turn clockwise for 0 - 3600 degrees (10 times 360)
    pub async fn cw(&mut self, step: u32) -> Result<(), String> {
        let command = format!("cw {}", step);
        let step_norm = step.min(3600).max(1);
        self.send_command(command.into())
            .await
            .and_then(|_| Ok(self.odometry.cw(step_norm)))
    }
    /// turn counter clockwise for 0 - 3600 degrees (10 times 360)
    pub async fn ccw(&mut self, step: u32) -> Result<(), String> {
        let step_norm = step.min(3600).max(1);
        let command = format!("ccw {}", step);
        self.send_command(command.into())
            .await
            .and_then(|_| Ok(self.odometry.ccw(step_norm)))
    }

    /// Go to a given position in the 3D space.
    ///
    /// - `x`, `y`, `z` 0 or (-)20 - (-)500 cm
    /// - `speed` speed in centimeter per second
    pub async fn go_to(&mut self, x: i32, y: i32, z: i32, speed: u8) -> Result<(), String> {
        let x_norm = (x == 0).then(|| 0).unwrap_or(x.min(500).max(20));
        let y_norm = (y == 0).then(|| 0).unwrap_or(y.min(500).max(20));
        let z_norm = (z == 0).then(|| 0).unwrap_or(z.min(500).max(20));
        let speed_norm = speed.min(100).max(10);
        let command = format!("go {} {} {} {}", x_norm, y_norm, z_norm, speed_norm);
        println!("{}", command);
        self.send_command(command.into()).await
    }

    /// Moves in a curve parsing the first point to the second point in the shortest path.
    ///
    /// The radius could not be to large and the distance cold not exceed the 500 cm
    /// the minimal distance to go is 0 or 20cm on `x`,`y`,`z`
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
        let x1_norm = (x1 == 0).then(|| 0).unwrap_or(x1.min(500).max(20));
        let y1_norm = (y1 == 0).then(|| 0).unwrap_or(y1.min(500).max(20));
        let z1_norm = (z1 == 0).then(|| 0).unwrap_or(z1.min(500).max(20));
        let x2_norm = (x2 == 0).then(|| 0).unwrap_or(x2.min(500).max(20));
        let y2_norm = (y2 == 0).then(|| 0).unwrap_or(y2.min(500).max(20));
        let z2_norm = (z2 == 0).then(|| 0).unwrap_or(z2.min(500).max(20));
        let speed_norm = speed.min(100).max(10);
        let command = format!(
            "curve {} {} {} {} {} {} {}",
            x1_norm, y1_norm, z1_norm, x2_norm, y2_norm, z2_norm, speed_norm
        );
        self.send_command(command.into()).await
    }

    /// set the speed for the forward, backward, right, left, up, down motion
    pub async fn speed(&self, speed: u8) -> Result<(), String> {
        println!("speed");
        let normalized_speed = speed.min(100).max(10);
        let command = format!("speed {}", normalized_speed);
        self.send_command(command.into()).await
    }
}
