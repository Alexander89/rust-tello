//! # Tello drone
//!
//! There are two interfaces for the tello drone. The text based and a
//! non-public interface, used by the native app. The guys from the
//! [tellopilots forum](https://tellopilots.com/) did an awesome job by
//! reverse engineer this interface and support other public repositories
//! for go, python...
//!
//! This library combines the network protocol to communicate with the drone and get
//! available meta data additionally and a remote-control framework is available to
//! simplify the wiring to the keyboard or an joystick.
//!
//! In the sources you will find an example, how to create a SDL-Ui and use
//! the keyboard to control the drone. You can run it with `cargo run --example fly`
//!
//! **Please keep in mind, advanced maneuvers require a bright environment. (Flip, Bounce, ...)**
//!
//! ## Communication
//!
//! When the drone gets an enable package (`drone.connect(11111);`), the Tello drone
//! send data on two UDP channels. A the command channel (port: 8889) and B the
//! video channel (default: port: 11111). In the AP mode, the drone will appear with
//! the default ip 192.168.10.1. All send calls are done synchronously.
//! To receive the data, you have to poll the drone. Here is an example:
//!
//!
//! ### Example
//!
//! ```
//! use tello::{Drone, Message, Package, PackageData, ResponseMsg};
//! use std::time::Duration;
//!
//! fn main() -> Result<(), String> {
//!     let mut drone = Drone::new("192.168.10.1:8889");
//!     drone.connect(11111);
//!     loop {
//!         if let Some(msg) = drone.poll() {
//!             match msg {
//!                 Message::Data(Package {data: PackageData::FlightData(d), ..}) => {
//!                     println!("battery {}", d.battery_percentage);
//!                 }
//!                 Message::Frame(frame_id, data) => {
//!                     println!("frame {} {:?}", frame_id, data);
//!                 }
//!                 Message::Response(ResponseMsg::Connected(_)) => {
//!                     println!("connected");
//!                     drone.throw_and_go().unwrap();
//!                 }
//!                 _ => ()
//!             }
//!         }
//!         ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
//!     }
//! }
//! ```
//!
//! ## Remote control
//!
//! The poll is not only receiving messages from the drone, it will also send some default-settings,
//! replies with acknowledgements, triggers the key frames or send the remote-control state for the
//! live move commands.
//!
//! The Drone contains a rc_state to manipulate the movement. e.g.: `drone.rc_state.go_down()`,
//! `drone.rc_state.go_forward_back(-0.7)`
//!
//! The following example is opening a window with SDL, handles the keyboard inputs and shows how to connect a
//! game pad or joystick.
//!
//!
//! ### Examples
//!
//! ```
//! use sdl2::event::Event;
//! use sdl2::keyboard::Keycode;
//! use tello::{Drone, Message, Package, PackageData, ResponseMsg};
//! use std::time::Duration;
//!
//! fn main() -> Result<(), String> {
//!     let mut drone = Drone::new("192.168.10.1:8889");
//!     drone.connect(11111);
//!
//!     let sdl_context = sdl2::init()?;
//!     let video_subsystem = sdl_context.video()?;
//!     let window = video_subsystem.window("TELLO drone", 1280, 720).build().unwrap();
//!     let mut canvas = window.into_canvas().build().unwrap();
//!
//!     let mut event_pump = sdl_context.event_pump()?;
//!     'running: loop {
//!         // draw some stuff
//!         canvas.clear();
//!         // [...]
//!
//!         // handle input from a keyboard or something like a game-pad
//!         // ue the keyboard events
//!         for event in event_pump.poll_iter() {
//!             match event {
//!                 Event::Quit { .. }
//!                 | Event::KeyDown { keycode: Some(Keycode::Escape), .. } =>
//!                     break 'running,
//!                 Event::KeyDown { keycode: Some(Keycode::K), .. } =>
//!                     drone.take_off().unwrap(),
//!                 Event::KeyDown { keycode: Some(Keycode::L), .. } =>
//!                     drone.land().unwrap(),
//!                 Event::KeyDown { keycode: Some(Keycode::A), .. } =>
//!                     drone.rc_state.go_left(),
//!                 Event::KeyDown { keycode: Some(Keycode::D), .. } =>
//!                     drone.rc_state.go_right(),
//!                 Event::KeyUp { keycode: Some(Keycode::A), .. }
//!                 | Event::KeyUp { keycode: Some(Keycode::D), .. } =>
//!                     drone.rc_state.stop_left_right(),
//!                 //...
//!             }
//!         }
//!
//!         // or use a game pad (range from -1 to 1)
//!         // drone.rc_state.go_left_right(dummy_joystick.axis.1);
//!         // drone.rc_state.go_forward_back(dummy_joystick.axis.2);
//!         // drone.rc_state.go_up_down(dummy_joystick.axis.3);
//!         // drone.rc_state.turn(dummy_joystick.axis.4);
//!
//!         // the poll will send the move command to the drone
//!         drone.poll();
//!
//!         canvas.present();
//!         ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
//!     }
//! }
//! ```

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::prelude::*;
use crc::{crc16, crc8};
use drone_state::{FlightData, LightInfo, LogMessage, WifiInfo};
use std::convert::TryFrom;
use std::io::{Cursor, Read, Write, Seek, SeekFrom};
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::SystemTime;

mod crc;
mod drone_state;
mod rc_state;

pub use drone_state::DroneMeta;
pub use rc_state::RCState;

static SEQ_NO: AtomicU16 = AtomicU16::new(1);

type Result = std::result::Result<(), ()>;

/// The video data itself is just H264 encoded YUV420p
#[derive(Debug, Clone)]
struct VideoSettings {
    pub port: u16,
    pub enabled: bool,
    pub mode: VideoMode,
    pub level: u8,
    pub encoding_rate: u8,
    pub last_video_poll: SystemTime,
    pub last_frame_id: u8,
    pub frame_counter_overflow: u32,
}

/// Main connection and controller for the drone
#[derive(Debug)]
pub struct Drone {
    socket: UdpSocket,
    video_socket: Option<UdpSocket>,
    video: VideoSettings,
    last_stick_command: SystemTime,

    /// remote control values to control the drone
    pub rc_state: RCState,

    /// current meta data from the drone
    pub drone_meta: DroneMeta,

    /// used to query some metadata delayed after connecting
    status_counter: u32,
}

const START_OF_PACKET: u8 = 0xcc;

/// known Command ids. Not all of them are implemented.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
pub enum CommandIds {
    Undefined = 0x0000,
    SsidMsg = 0x0011,
    SsidCmd = 0x0012,
    SsidPasswordMsg = 0x0013,
    SsidPasswordCmd = 0x0014,
    WifiRegionMsg = 0x0015,
    WifiRegionCmd = 0x0016,
    WifiMsg = 0x001a,
    VideoEncoderRateCmd = 0x0020,
    VideoDynAdjRateCmd = 0x0021,
    EisCmd = 0x0024,
    VideoStartCmd = 0x0025,
    VideoRateQuery = 0x0028,
    TakePictureCommand = 0x0030,
    VideoModeCmd = 0x0031,
    VideoRecordCmd = 0x0032,
    ExposureCmd = 0x0034,
    LightMsg = 0x0035,
    JpegQualityMsg = 0x0037,
    Error1Msg = 0x0043,
    Error2Msg = 0x0044,
    VersionMsg = 0x0045,
    TimeCmd = 0x0046,
    ActivationTimeMsg = 0x0047,
    LoaderVersionMsg = 0x0049,
    StickCmd = 0x0050,
    TakeoffCmd = 0x0054,
    LandCmd = 0x0055,
    FlightMsg = 0x0056,
    AltLimitCmd = 0x0058,
    FlipCmd = 0x005c,
    ThrowAndGoCmd = 0x005d,
    PalmLandCmd = 0x005e,
    TelloCmdFileSize = 0x0062,
    TelloCmdFileData = 0x0063,
    TelloCmdFileComplete = 0x0064,
    SmartVideoCmd = 0x0080,
    SmartVideoStatusMsg = 0x0081,
    LogHeaderMsg = 0x1050,
    LogDataMsg = 0x1051,
    LogConfigMsg = 0x1052,
    BounceCmd = 0x1053,
    CalibrateCmd = 0x1054,
    LowBatThresholdCmd = 0x1055,
    AltLimitMsg = 0x1056,
    LowBatThresholdMsg = 0x1057,
    AttLimitCmd = 0x1058,
    AttLimitMsg = 0x1059,
}

impl From<u16> for CommandIds {
    fn from(value: u16) -> CommandIds {
        match value {
            0x0011 => CommandIds::SsidMsg,
            0x0012 => CommandIds::SsidCmd,
            0x0013 => CommandIds::SsidPasswordMsg,
            0x0014 => CommandIds::SsidPasswordCmd,
            0x0015 => CommandIds::WifiRegionMsg,
            0x0016 => CommandIds::WifiRegionCmd,
            0x001a => CommandIds::WifiMsg,
            0x0020 => CommandIds::VideoEncoderRateCmd,
            0x0021 => CommandIds::VideoDynAdjRateCmd,
            0x0024 => CommandIds::EisCmd,
            0x0025 => CommandIds::VideoStartCmd,
            0x0028 => CommandIds::VideoRateQuery,
            0x0030 => CommandIds::TakePictureCommand,
            0x0031 => CommandIds::VideoModeCmd,
            0x0032 => CommandIds::VideoRecordCmd,
            0x0034 => CommandIds::ExposureCmd,
            0x0035 => CommandIds::LightMsg,
            0x0037 => CommandIds::JpegQualityMsg,
            0x0043 => CommandIds::Error1Msg,
            0x0044 => CommandIds::Error2Msg,
            0x0045 => CommandIds::VersionMsg,
            0x0046 => CommandIds::TimeCmd,
            0x0047 => CommandIds::ActivationTimeMsg,
            0x0049 => CommandIds::LoaderVersionMsg,
            0x0050 => CommandIds::StickCmd,
            0x0054 => CommandIds::TakeoffCmd,
            0x0055 => CommandIds::LandCmd,
            0x0056 => CommandIds::FlightMsg,
            0x0058 => CommandIds::AltLimitCmd,
            0x005c => CommandIds::FlipCmd,
            0x005d => CommandIds::ThrowAndGoCmd,
            0x005e => CommandIds::PalmLandCmd,
            0x0062 => CommandIds::TelloCmdFileSize,
            0x0063 => CommandIds::TelloCmdFileData,
            0x0064 => CommandIds::TelloCmdFileComplete,
            0x0080 => CommandIds::SmartVideoCmd,
            0x0081 => CommandIds::SmartVideoStatusMsg,
            0x1050 => CommandIds::LogHeaderMsg,
            0x1051 => CommandIds::LogDataMsg,
            0x1052 => CommandIds::LogConfigMsg,
            0x1053 => CommandIds::BounceCmd,
            0x1054 => CommandIds::CalibrateCmd,
            0x1055 => CommandIds::LowBatThresholdCmd,
            0x1056 => CommandIds::AltLimitMsg,
            0x1057 => CommandIds::LowBatThresholdMsg,
            0x1058 => CommandIds::AttLimitCmd,
            0x1059 => CommandIds::AttLimitMsg,
            _ => CommandIds::Undefined,
        }
    }
}
/// unformatted response from the drone.
#[derive(Debug, Clone)]
pub enum ResponseMsg {
    Connected(String),
    UnknownCommand(CommandIds),
}

/// The package type bitmask discripe the payload and how the drone should behave.
/// More info are available on the https://tellopilots.com webpage
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum PackageTypes {
    X48 = 0x48,
    X50 = 0x50,
    X60 = 0x60,
    X70 = 0x70,
    X68 = 0x68,
}

/// Flip commands taken from Go version of code
pub enum Flip {
    /// flips forward.
    Forward = 0,
    /// flips left.
    Left = 1,
    /// flips backwards.
    Back = 2,
    /// flips to the right.
    Right = 3,
    /// flips forwards and to the left.
    ForwardLeft = 4,
    /// flips backwards and to the left.
    BackLeft = 5,
    /// flips backwards and to the right.
    BackRight = 6,
    /// flips forwards and to the right.
    ForwardRight = 7,
}

/// available modes for the tello drone
#[derive(Debug, Clone)]
pub enum VideoMode {
    M960x720 = 0,
    M1280x720 = 1,
}

impl Drone {
    /// create a new drone and and listen to the Response port 8889
    /// this struct implements a number of commands to control the drone
    ///
    /// After connection to the drone it is very important to poll the drone at least with 20Hz.
    /// This will acknowledge some messages and parse the state of the drone.
    ///
    /// # Example
    ///
    /// ```
    /// let mut drone = Drone::new("192.168.10.1:8889");
    /// drone.connect(11111);
    /// // wait for the connection
    /// drone.take_off();
    /// ```
    pub fn new(ip: &str) -> Drone {
        let socket = UdpSocket::bind(&SocketAddr::from(([0, 0, 0, 0], 8889))).expect("couldn't bind to command address");
        socket.set_nonblocking(true).unwrap();
        socket.connect(ip).expect("connect command socket failed");

        let video = VideoSettings {
            port: 0,
            enabled: false,
            mode: VideoMode::M960x720,
            level: 1,
            encoding_rate: 4,
            last_video_poll: SystemTime::now(),
            last_frame_id: 0,
            frame_counter_overflow: 0,
        };

        let rc_state = RCState::default();
        let drone_meta = DroneMeta::default();

        Drone {
            socket,
            video_socket: None,
            video,
            status_counter: 0,
            last_stick_command: SystemTime::now(),
            rc_state,
            drone_meta,
        }
    }

    /// Connect to the drone and inform the drone on with port you are ready to receive the video-stream
    ///
    /// The Video stream do not start automatically. You have to start it with
    /// `drone.start_video()` and pool every key-frame with an additional `drone.start_video()` call.
    pub fn connect(&mut self, video_port: u16) -> usize {
        let mut data = b"conn_req:  ".to_vec();
        let mut cur = Cursor::new(&mut data);
        cur.set_position(9);
        cur.write_u16::<LittleEndian>(video_port).unwrap();
        self.video.port = video_port;
        self.start_video().unwrap();

        let video_socket = UdpSocket::bind(&SocketAddr::from(([0, 0, 0, 0], self.video.port))).expect("couldn't bind to video address");
        video_socket.set_nonblocking(true).unwrap();
        self.video_socket = Some(video_socket);

        self.socket.send(&data).expect("network should be usable")
    }

    /// convert the command into a Vec<u8> and send it to the drone.
    /// this is mostly for internal purposes, but you can implement missing commands your self
    pub fn send(&self, command: UdpCommand) -> Result {
        let data: Vec<u8> = command.into();

        if self.socket.send(&data).is_ok() {
            Ok(())
        } else {
            Err(())
        }
    }

    /// when the drone send the current log stats, it is required to ack this.
    /// The logic is implemented in the poll function.
    fn send_ack_log(&self, id: u16) -> Result {
        let mut cmd = UdpCommand::new_with_zero_sqn(CommandIds::LogHeaderMsg, PackageTypes::X50);
        cmd.write_u16(id);
        self.send(cmd)
    }

    /// if there are some data in the udp-socket, all of one frame are collected and returned as UDP-Package
    fn receive_video_frame(&mut self)-> Option<Message> {
        let mut read_buf = [0; 1440];
        let socket = self.video_socket.as_ref().unwrap();

        socket.set_nonblocking(true).unwrap();
        if let Ok(received) = socket.recv(&mut read_buf) {

            let active_frame_id = read_buf[0];

            if active_frame_id < 100 && active_frame_id < self.video.last_frame_id {
                self.video.frame_counter_overflow += 1;
            }
            self.video.last_frame_id = active_frame_id;

            let mut sqn = read_buf[1];
            let mut frame_buffer = read_buf[2..received].to_owned();

            // should start with 0. otherwise delete frame package
            if sqn != 0 {
                return None
            }

            socket.set_nonblocking(false).unwrap();
            'recVideo : loop {
                if sqn >= 120 {
                    let frame_id: u32 = active_frame_id as u32 + 255 * self.video.frame_counter_overflow;
                    break 'recVideo Some( Message::Frame(frame_id, frame_buffer) )
                }
                if let Ok(received) = socket.recv(&mut read_buf) {
                    let frame_id = read_buf[0];
                    if frame_id != active_frame_id {
                        // drop frame to stop data mess
                        break 'recVideo None
                    }

                    sqn = read_buf[1];
                    let mut data = read_buf[2..received].to_owned();


                    frame_buffer.append(&mut data);

                } else {
                    break 'recVideo None
                }
            }
        } else {
            None
        }
    }

    /// poll data from drone and send common data to the drone
    /// - every 33 millis, the sick command is send to the drone
    /// - every 1 sec, a key-frame is requested from the drone
    /// - logMessage packages are replied immediately with an ack package
    /// - dateTime packages are replied immediately with the local SystemTime
    /// - after the third status message some default data are send to the drone
    ///
    /// To receive a smooth video stream, you should poll at least 35 times per second
    pub fn poll(&mut self) -> Option<Message> {
        let now = SystemTime::now();

        let delta = now.duration_since(self.last_stick_command).unwrap();
        if delta.as_millis() > 1000 / 30 {
            let (pitch, nick, roll, yaw, fast) = self.rc_state.get_stick_parameter();
            self.send_stick(pitch, nick, roll, yaw, fast).unwrap();
            self.last_stick_command = now.clone();
        }

        // poll I-Frame every second and receive udp frame data
        if self.video.enabled {
            let delta = now.duration_since(self.video.last_video_poll).unwrap();
            if delta.as_secs() > 1 {
                self.video.last_video_poll = now;
                self.poll_key_frame().unwrap();
            }
            if self.video_socket.is_some() {
                let frame = self.receive_video_frame();
                if frame.is_some() {
                    return frame;
                }
            }
        }

        // receive and process data on command socket
        let mut read_buf = [0; 1440];
        if let Ok(received) = self.socket.recv(&mut read_buf) {
            let data = read_buf[..received].to_vec();
            match Message::try_from(data) {
                Ok(msg) => {
                    match &msg {
                        Message::Response(ResponseMsg::Connected(_)) => self.status_counter = 0,
                        Message::Data(Package {
                            data: PackageData::LogMessage(log),
                            ..
                        }) => self.send_ack_log(log.id).unwrap(),
                        Message::Data(Package { cmd, .. }) if *cmd == CommandIds::TimeCmd => {
                            self.send_date_time().unwrap()
                        }
                        Message::Data(Package { cmd, data, .. })
                            if *cmd == CommandIds::FlightMsg =>
                        {
                            self.drone_meta.update(&data);

                            self.status_counter += 1;
                            if self.status_counter == 3 {
                                self.get_version().unwrap();
                                self.set_video_bitrate(4).unwrap();
                                self.get_alt_limit().unwrap();
                                self.get_battery_threshold().unwrap();
                                self.get_att_angle().unwrap();
                                self.get_region().unwrap();
                                self.set_exposure(2).unwrap();
                            };
                        }
                        Message::Data(Package { data, .. }) => {
                            self.drone_meta.update(&data);
                        }
                        _ => (),
                    };

                    Some(msg)
                }
                Err(_e) => None,
            }
        } else {
            None
        }
    }
}

impl Drone {
    pub fn take_off(&self) -> Result {
        self.send(UdpCommand::new(
            CommandIds::TakeoffCmd,
            PackageTypes::X68,
        ))
    }
    pub fn throw_and_go(&self) -> Result {
        let mut cmd = UdpCommand::new(CommandIds::ThrowAndGoCmd, PackageTypes::X48);
        cmd.write_u8(0);
        self.send(cmd)
    }
    pub fn land(&self) -> Result {
        let mut command = UdpCommand::new(CommandIds::LandCmd, PackageTypes::X68);
        command.write_u8(0x00);
        self.send(command)
    }
    pub fn stop_land(&self) -> Result {
        let mut command = UdpCommand::new(CommandIds::LandCmd, PackageTypes::X68);
        command.write_u8(0x00);
        self.send(command)
    }
    pub fn palm_land(&self) -> Result {
        let mut cmd = UdpCommand::new(CommandIds::PalmLandCmd, PackageTypes::X68);
        cmd.write_u8(0);
        self.send(cmd)
    }

    pub fn flip(&self, direction: Flip) -> Result {
        let mut cmd = UdpCommand::new_with_zero_sqn(CommandIds::FlipCmd, PackageTypes::X70);
        cmd.write_u8(direction as u8);
        self.send(cmd)
    }
    pub fn bounce(&self) -> Result {
        let mut cmd = UdpCommand::new(CommandIds::BounceCmd, PackageTypes::X68);
        cmd.write_u8(0x30);
        self.send(cmd)
    }
    pub fn bounce_stop(&self) -> Result {
        let mut cmd = UdpCommand::new(CommandIds::BounceCmd, PackageTypes::X68);
        cmd.write_u8(0x31);
        self.send(cmd)
    }

    pub fn get_version(&self) -> Result {
        self.send(UdpCommand::new(
            CommandIds::VersionMsg,
            PackageTypes::X48,
        ))
    }
    pub fn get_alt_limit(&self) -> Result {
        self.send(UdpCommand::new(
            CommandIds::AltLimitMsg,
            PackageTypes::X68,
        ))
    }
    pub fn set_alt_limit(&self, limit: u8) -> Result {
        let mut cmd = UdpCommand::new(CommandIds::AltLimitCmd, PackageTypes::X68);
        cmd.write_u8(limit);
        cmd.write_u8(0);
        self.send(cmd)
    }
    pub fn get_att_angle(&self) -> Result {
        self.send(UdpCommand::new(
            CommandIds::AttLimitMsg,
            PackageTypes::X68,
        ))
    }
    pub fn set_att_angle(&self) -> Result {
        let mut cmd = UdpCommand::new(CommandIds::AttLimitCmd, PackageTypes::X68);
        cmd.write_u8(0);
        cmd.write_u8(0);
        // TODO set angle correct
        // pkt.add_byte( int(float_to_hex(float(limit))[4:6], 16) ) # 'attitude limit' formatted in float of 4 bytes
        cmd.write_u8(10);
        cmd.write_u8(0x41);
        self.send(cmd)
    }

    pub fn get_battery_threshold(&self) -> Result {
        self.send(UdpCommand::new(
            CommandIds::LowBatThresholdMsg,
            PackageTypes::X68,
        ))
    }
    pub fn set_battery_threshold(&self, threshold: u8) -> Result {
        let mut cmd = UdpCommand::new(CommandIds::LowBatThresholdCmd, PackageTypes::X68);
        cmd.write_u8(threshold);
        self.send(cmd)
    }

    pub fn get_region(&self) -> Result {
        self.send(UdpCommand::new(
            CommandIds::WifiRegionCmd,
            PackageTypes::X48,
        ))
    }

    /// send the stick command via udp to the drone
    ///
    /// pitch up/down -1 -> 1
    /// nick forward/backward -1 -> 1
    /// roll right/left -1 -> 1
    /// yaw cw/ccw -1 -> 1
    pub fn send_stick(&self, pitch: f32, nick: f32, roll: f32, yaw: f32, fast: bool) -> Result {
        let mut cmd = UdpCommand::new_with_zero_sqn(CommandIds::StickCmd, PackageTypes::X60);

        // RightX center=1024 left =364 right =-364
        let pitch_u = (1024.0 + 660.0 * pitch) as i64;

        // RightY down =364 up =-364
        let nick_u = (1024.0 + 660.0 * nick) as i64;

        // LeftY down =364 up =-364
        let roll_u = (1024.0 + 660.0 * roll) as i64;

        // LeftX left =364 right =-364
        let yaw_u = (1024.0 + 660.0 * yaw) as i64;

        // speed control
        let throttle_u = if fast { 1i64 } else { 0i64 };

        // create axis package
        let packed_axis: i64 = (roll_u & 0x7FF)
            | (nick_u & 0x7FF) << 11
            | (pitch_u & 0x7FF) << 22
            | (yaw_u & 0x7FF) << 33
            | throttle_u << 44;

        // println!("p {:} n {:} r {:} y {:} t {:} => {:x}", pitch_u & 0x7FF, nick_u& 0x7FF, roll_u& 0x7FF, yaw_u& 0x7FF, throttle_u, packed_axis);

        cmd.write_u8(((packed_axis) & 0xFF) as u8);
        cmd.write_u8(((packed_axis >> 8) & 0xFF) as u8);
        cmd.write_u8(((packed_axis >> 16) & 0xFF) as u8);
        cmd.write_u8(((packed_axis >> 24) & 0xFF) as u8);
        cmd.write_u8(((packed_axis >> 32) & 0xFF) as u8);
        cmd.write_u8(((packed_axis >> 40) & 0xFF) as u8);

        self.send(Drone::add_time(cmd))
    }

    /// SendDateTime sends the current date/time to the drone.
    pub fn send_date_time(&self) -> Result {
        let command = UdpCommand::new(CommandIds::TimeCmd, PackageTypes::X50);
        self.send(Drone::add_date_time(command))
    }

    pub fn add_time(mut command: UdpCommand) -> UdpCommand {
        let now = Local::now();
        let millis = now.nanosecond() / 1_000_000;
        command.write_u8(now.hour() as u8);
        command.write_u8(now.minute() as u8);
        command.write_u8(now.second() as u8);
        command.write_u16(millis as u16);
        command
    }

    pub fn add_date_time(mut command: UdpCommand) -> UdpCommand {
        let now = Local::now();
        let millis = now.nanosecond() / 1_000_000;
        command.write_u8(0);
        command.write_u16(now.year() as u16);
        command.write_u16(now.month() as u16);
        command.write_u16(now.day() as u16);
        command.write_u16(now.hour() as u16);
        command.write_u16(now.minute() as u16);
        command.write_u16(now.second() as u16);
        command.write_u16(millis as u16);
        command
    }
}

impl Drone {
    /// start_video starts the streaming and requests the info (SPS/PPS) for the video stream.
    ///
    /// Video-metadata:
    /// e.g.: caps = video/x-h264, stream-format=(string)avc, width=(int)960, height=(int)720, framerate=(fraction)0/1, interlace-mode=(string)progressive, chroma-format=(string)4:2:0, bit-depth-luma=(uint)8, bit-depth-chroma=(uint)8, parsed=(boolean)true, alignment=(string)au, profile=(string)main, level=(string)4, codec_data=(buffer)014d4028ffe10009674d402895a03c05b901000468ee3880
    ///
    /// # Examples
    /// ```no_run
    /// let mut drone = Drone::new("192.168.10.1:8889");
    /// drone.connect(11111);
    /// // ...
    /// drone.start_video().unwrap();
    /// ```
    pub fn start_video(&mut self) -> Result {
        self.video.enabled = true;
        self.video.last_video_poll = SystemTime::now();
        self.send(UdpCommand::new_with_zero_sqn(CommandIds::VideoStartCmd, PackageTypes::X60))
    }

    /// Same as start_video(), but a better name to poll the (SPS/PPS) for the video stream.
    ///
    /// This is automatically called in the poll function every second.
    pub fn poll_key_frame(&mut self) -> Result {
        self.start_video()
    }

    /// Set the video mode to 960x720 4:3 video, or 1280x720 16:9 zoomed video.
    /// 4:3 has a wider field of view (both vertically and horizontally), 16:9 is crisper.
    ///
    /// # Examples
    /// ```no_run
    /// let mut drone = Drone::new("192.168.10.1:8889");
    /// drone.connect(11111);
    /// // ...
    /// drone.set_video_mode(VideoMode::M960x720).unwrap();
    /// ```
    pub fn set_video_mode(&mut self, mode: VideoMode) -> Result {
        self.video.mode = mode.clone();
        let mut cmd =
            UdpCommand::new_with_zero_sqn(CommandIds::VideoStartCmd, PackageTypes::X68);
        cmd.write_u8(mode as u8);
        self.send(cmd)
    }

    /// Set the camera exposure level.
    /// param level: it can be 0, 1 or 2
    ///
    /// # Examples
    /// ```no_run
    /// let mut drone = Drone::new("192.168.10.1:8889");
    /// drone.connect(11111);
    /// // ...
    /// drone.set_exposure(2).unwrap();
    /// ```
    pub fn set_exposure(&mut self, level: u8) -> Result {
        let mut cmd = UdpCommand::new(CommandIds::ExposureCmd, PackageTypes::X48);
        cmd.write_u8(level);
        self.send(cmd)
    }

    /// set the video encoder rate for the camera.
    /// param rate: TODO: unknown
    ///
    /// # Examples
    /// ```no_run
    /// let mut drone = Drone::new("192.168.10.1:8889");
    /// drone.connect(11111);
    /// // ...
    /// drone.set_video_bitrate(3).unwrap();
    /// ```
    pub fn set_video_bitrate(&mut self, rate: u8) -> Result {
        self.video.encoding_rate = rate;
        let mut cmd = UdpCommand::new(CommandIds::VideoEncoderRateCmd, PackageTypes::X68);
        cmd.write_u8(rate);
        self.send(cmd)
    }

    /// take a single picture and provide it to download it.
    ///
    /// # Examples
    /// ```no_run
    /// let mut drone = Drone::new("192.168.10.1:8889");
    /// drone.connect(11111);
    /// // ...
    /// drone.take_picture(3).unwrap();
    ///
    /// @TODO: download image
    /// ```
    pub fn take_picture(&self) -> Result {
        self.send(UdpCommand::new(
            CommandIds::TakePictureCommand,
            PackageTypes::X68,
        ))
    }
}

/// wrapper to generate Udp Commands to send them to the drone.
///
/// It is public, to enable users to implement missing commands
#[derive(Debug, Clone)]
pub struct UdpCommand {
    cmd: CommandIds,
    pkt_type: PackageTypes,
    zero_sqn: bool,
    inner: Vec<u8>,
}

impl UdpCommand {
    /// create a new command, prepare the header to send out the command
    pub fn new(cmd: CommandIds, pkt_type: PackageTypes) -> UdpCommand {
        UdpCommand {
            cmd,
            pkt_type,
            zero_sqn: false,
            inner: Vec::new(),
        }
    }
    pub fn new_with_zero_sqn(cmd: CommandIds, pkt_type: PackageTypes) -> UdpCommand {
        UdpCommand {
            cmd,
            pkt_type,
            zero_sqn: true,
            inner: Vec::new(),
        }
    }
}

impl UdpCommand {
    pub fn write(&mut self, bytes: &[u8]) {
        self.inner.append(&mut bytes.to_owned())
    }
    pub fn write_u8(&mut self, byte: u8) {
        self.inner.push(byte)
    }
    pub fn write_u16(&mut self, value: u16) {
        let mut cur = Cursor::new(&mut self.inner);
        cur.seek(SeekFrom::End(0)).expect("");
        cur.write_u16::<LittleEndian>(value).expect("");
    }
    pub fn write_u64(&mut self, value: u64) {
        let mut cur = Cursor::new(&mut self.inner);
        cur.seek(SeekFrom::End(0)).expect("");
        cur.write_u64::<LittleEndian>(value).expect("");
    }
}

impl Into<Vec<u8>> for UdpCommand {
    fn into(self) -> Vec<u8> {
        let mut data = {
            let lng = self.inner.len();
            let data: &[u8]= &self.inner;

            let mut cur = Cursor::new(Vec::new());
            cur.write_u8(START_OF_PACKET).expect("");
            cur.write_u16::<LittleEndian>((lng as u16 + 11) << 3).expect("");
            cur.write_u8(crc8(cur.clone().into_inner())).expect("");
            cur.write_u8(self.pkt_type as u8).expect("");
            cur.write_u16::<LittleEndian>(self.cmd as u16).expect("");

            if self.zero_sqn {
                cur.write_u16::<LittleEndian>(0).expect("");
            } else {
                let nr = SEQ_NO.fetch_add(1, Ordering::SeqCst);
                cur.write_u16::<LittleEndian>(nr).expect("");
            }

            if lng > 0 {
                cur.write_all(&data).unwrap();
            }

            cur.into_inner()
        };

        data
            .write_u16::<LittleEndian>(crc16(data.clone()))
            .expect("");

        data
    }
}

/// Data / command package received from the drone with parsed data (if supported and known)
#[derive(Debug, Clone)]
pub struct Package {
    pub cmd: CommandIds,
    pub size: u16,
    pub sq_nr: u16,
    pub data: PackageData,
}

/// Incoming message can be Data, a response from the drone or a VideoFrame
#[derive(Debug, Clone)]
pub enum Message {
    Data(Package),
    Response(ResponseMsg),
    Frame(u32, Vec<u8>),
}

impl TryFrom<Vec<u8>> for Message {
    type Error = String;

    fn try_from(data: Vec<u8>) -> std::result::Result<Self, Self::Error> {
        let mut cur = Cursor::new(data);
        if let Ok(START_OF_PACKET) = cur.read_u8() {
            let size = (cur.read_u16::<LittleEndian>().unwrap() >> 3) - 11;
            let _crc8 = cur.read_u8().unwrap();
            let _pkt_type = cur.read_u8().unwrap();
            let cmd = CommandIds::from(cur.read_u16::<LittleEndian>().unwrap());
            let sq_nr = cur.read_u16::<LittleEndian>().unwrap();
            let data = if size > 0 {
                let mut data: Vec<u8> = Vec::with_capacity(size as usize);
                cur.read_to_end(&mut data).unwrap();
                if data.len() >= 2 {
                    let _crc16: u16 =
                        (data.pop().unwrap() as u16) + ((data.pop().unwrap() as u16) << 8);
                }
                match cmd {
                    CommandIds::FlightMsg => PackageData::FlightData(FlightData::from(data)),
                    CommandIds::WifiMsg => PackageData::WifiInfo(WifiInfo::from(data)),
                    CommandIds::LightMsg => PackageData::LightInfo(LightInfo::from(data)),
                    CommandIds::VersionMsg => PackageData::Version(
                        String::from_utf8(data[1..].to_vec())
                            .expect("version is not valid")
                            .trim_matches(char::from(0))
                            .to_string(),
                    ),
                    CommandIds::AltLimitMsg => {
                        let mut c = Cursor::new(data);
                        let _ = c.read_u8().unwrap();
                        let h = c.read_u16::<LittleEndian>().unwrap();
                        PackageData::AtlInfo(h)
                    }

                    CommandIds::LogHeaderMsg => PackageData::LogMessage(LogMessage::from(data)),
                    _ => PackageData::Unknown(data),
                }
            } else {
                PackageData::NoData()
            };

            Ok(Message::Data(Package {
                cmd,
                size,
                sq_nr,
                data,
            }))
        } else {
            let data = cur.into_inner();
            if data[0..9].to_vec() == b"conn_ack:" {
                return Ok(Message::Response(ResponseMsg::Connected(
                    String::from_utf8(data).unwrap(),
                )));
            } else if data[0..16].to_vec() == b"unknown command:" {
                let mut cur = Cursor::new(data[17..].to_owned());
                let command = CommandIds::from(cur.read_u16::<LittleEndian>().unwrap());
                return Ok(Message::Response(ResponseMsg::UnknownCommand(command)));
            }

            let msg = String::from_utf8(data.clone()[0..5].to_vec()).unwrap_or_default();
            Err(format!("invalid package {:x?}", msg))
        }
    }
}

/// Parsed data from the drone.
#[derive(Debug, Clone)]
pub enum PackageData {
    NoData(),
    AtlInfo(u16),
    FlightData(FlightData),
    LightInfo(LightInfo),
    LogMessage(LogMessage),
    Version(String),
    WifiInfo(WifiInfo),
    Unknown(Vec<u8>),
}
