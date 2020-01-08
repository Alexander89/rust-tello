use crate::crc::{crc16, crc8};
use crate::drone_messages::{FlightData, LightInfo, LogMessage, WifiInfo};
use crate::rc_state::RCState;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::prelude::*;
use std::convert::TryFrom;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::net::{SocketAddr, UdpSocket};
use std::time::{SystemTime, Duration};

static mut SEQ_NO: u16 = 1;

type Result = std::result::Result<(), ()>;

#[derive(Debug, Clone)]
// The video data itself is just H264 encoded YUV420p
struct VideoSettings {
  pub port: u16,
  pub enabled: bool,
  pub mode: VideoMode,
  pub level: u8,
  pub encoding_rate: u8,
  pub last_video_poll: SystemTime,
}

#[derive(Debug)]
pub struct Command {
  socket: UdpSocket,
  video: VideoSettings,
}

pub const START_OF_PACKET: u8 = 0xcc;

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

#[derive(Debug, Clone)]
pub enum ResponseMsg {
  Connected(String),
  UnknownCommand(CommandIds),
}

#[repr(u8)]
pub enum PackageTypes {
  X48 = 0x48,
  X50 = 0x50,
  X60 = 0x60,
  X70 = 0x70,
  X68 = 0x68,
}

//Flip commands taken from Go version of code
pub enum Flip {
  //flips forward.
  Forward = 0,
  //flips left.
  Left = 1,
  //flips backwards.
  Back = 2,
  //flips to the right.
  Right = 3,
  //flips forwards and to the left.
  ForwardLeft = 4,
  //flips backwards and to the left.
  BackLeft = 5,
  //flips backwards and to the right.
  BackRight = 6,
  //flips forwards and to the right.
  ForwardRight = 7,
}

#[derive(Debug, Clone)]
pub enum VideoMode {
  M960x720 = 0,
  M1280x720 = 1
}

impl Command {
  pub fn new(ip: &str) -> Command {
    let bind_addr = SocketAddr::from(([0, 0, 0, 0], 8889));
    let socket = UdpSocket::bind(&bind_addr).expect("couldn't bind to command address");
    socket.set_nonblocking(true).unwrap();
    socket.connect(ip).expect("connect command socket failed");

    let video = VideoSettings {
      port: 0,
      enabled: false,
      mode: VideoMode::M960x720,
      level: 1,
      encoding_rate: 4,
      last_video_poll: SystemTime::now(),
    };

    Command { socket, video }
  }

  pub fn connect(&mut self, video_port: u16) -> usize {
    let mut data = b"conn_req:  ".to_vec();
    let mut cur = Cursor::new(&mut data);
    cur.set_position(9);
    cur.write_u16::<LittleEndian>(video_port).unwrap();
    self.video.port = video_port;
    println!("connect command {:?}", data);
    self.socket.send(&data).expect("network should be usable")
  }

  pub fn send(&self, command: UdpCommand) -> Result {
    let data: Vec<u8> = command.into();

    println!("send command {:?}", data.clone());
    if self.socket.send(&data).is_ok() {
      Ok(())
    } else {
      Err(())
    }
  }

  fn send_ack_log(&self, id: u16) -> Result {
    let mut cmd = UdpCommand::new_with_zero_sqn(CommandIds::LogHeaderMsg, PackageTypes::X50, 2);
    cmd.write_u16(id);
    self.send(cmd)
  }

  pub fn poll(&mut self) -> Option<Message> {
    let mut meta_buf = [0; 1440];

    // poll I-Frame  every second
    if self.video.enabled {
      let now = SystemTime::now();
      let delta = now.duration_since(self.video.last_video_poll).unwrap();
      if delta.as_secs() > 1 {
        self.video.last_video_poll = now;
        self.start_video().unwrap();
      }
    }

    if let Ok(received) = self.socket.recv(&mut meta_buf) {
      let data = meta_buf[..received].to_vec();
      match Message::try_from(data) {
        Ok(msg) => {
          match msg.clone() {
            Message::Data(Package {
              data: PackageData::LogMessage(log),
              ..
            }) => self.send_ack_log(log.id).unwrap(),
            Message::Data(Package { cmd, .. }) if cmd == CommandIds::TimeCmd => {
              self.send_date_time().unwrap()
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

impl Command {
  pub fn start_engines(&self, rc_state: &mut RCState) {
    rc_state.start_engines();
  }

  pub fn take_off(&self) -> Result {
    self.send(UdpCommand::new(CommandIds::TakeoffCmd, PackageTypes::X68, 0))
  }
  pub fn throw_and_go(&self) -> Result {
    let mut cmd = UdpCommand::new(CommandIds::ThrowAndGoCmd, PackageTypes::X48, 1);
    cmd.write_u8(0);
    self.send(cmd)
  }
  pub fn land(&self) -> Result {
    let mut command = UdpCommand::new(CommandIds::LandCmd, PackageTypes::X68, 1);
    command.write_u8(0x00);
    self.send(command)
  }
  pub fn stop_land(&self) -> Result {
    let mut command = UdpCommand::new(CommandIds::LandCmd, PackageTypes::X68, 1);
    command.write_u8(0x00);
    self.send(command)
  }
  pub fn palm_land(&self) -> Result {
    let mut cmd = UdpCommand::new(CommandIds::PalmLandCmd, PackageTypes::X68, 1);
    cmd.write_u8(0);
    self.send(cmd)
  }


  pub fn flip(&self, direction: Flip) -> Result {
    let mut cmd = UdpCommand::new_with_zero_sqn(CommandIds::FlipCmd, PackageTypes::X70, 1);
    cmd.write_u8(direction as u8);
    self.send(cmd)
  }
  pub fn bounce(&self) -> Result {
    let mut cmd = UdpCommand::new(CommandIds::BounceCmd, PackageTypes::X68, 1);
    cmd.write_u8(0x30);
    self.send(cmd)
  }
  pub fn bounce_stop(&self) -> Result {
    let mut cmd = UdpCommand::new(CommandIds::BounceCmd, PackageTypes::X68, 1);
    cmd.write_u8(0x31);
    self.send(cmd)
  }

  pub fn get_version(&self) -> Result {
    self.send(UdpCommand::new(CommandIds::VersionMsg, PackageTypes::X48, 0))
  }
  pub fn get_alt_limit(&self) -> Result {
    self.send(UdpCommand::new(CommandIds::AltLimitMsg, PackageTypes::X68, 0))
  }
  pub fn set_alt_limit(&self, limit: u8) -> Result {
    let mut cmd = UdpCommand::new(CommandIds::AltLimitCmd, PackageTypes::X68, 2);
    cmd.write_u8(30);
    cmd.write_u8(0);
    self.send(cmd)
  }
  pub fn get_att_angle(&self) -> Result {
    self.send(UdpCommand::new(CommandIds::AttLimitMsg, PackageTypes::X68, 0))
  }
  pub fn set_att_angle(&self) -> Result {
    let mut cmd = UdpCommand::new(CommandIds::AttLimitCmd, PackageTypes::X68, 4);
    cmd.write_u8(0);
    cmd.write_u8(0);
    // TODO set angle correct
    // pkt.add_byte( int(float_to_hex(float(limit))[4:6], 16) ) # 'attitude limit' formatted in float of 4 bytes
    cmd.write_u8(10);
    cmd.write_u8(0x41);
    self.send(cmd)
  }

  pub fn get_battery_threshold(&self) -> Result {
    self.send(UdpCommand::new(CommandIds::LowBatThresholdMsg, PackageTypes::X68, 0))
  }
  pub fn set_battery_threshold(&self, threshold: u8) -> Result {
    let mut cmd = UdpCommand::new(CommandIds::LowBatThresholdCmd, PackageTypes::X68, 1);
    cmd.write_u8(threshold);
    self.send(cmd)
  }


  pub fn get_region(&self) -> Result {
    self.send(UdpCommand::new(CommandIds::WifiRegionCmd, PackageTypes::X48, 0))
  }



  // pitch up/down -1 -> 1
  // nick forward/backward -1 -> 1
  // roll right/left -1 -> 1
  // yaw cw/ccw -1 -> 1
  pub fn send_stick(&self, pitch: f32, nick: f32, roll: f32, yaw: f32, fast: bool) -> Result {
    let mut cmd = UdpCommand::new_with_zero_sqn(CommandIds::StickCmd, PackageTypes::X60, 11);

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

    let cmd = Command::add_time(cmd);
    self.send(cmd)
  }
  // SendDateTime sends the current date/time to the drone.
  pub fn send_date_time(&self) -> Result {
    let command = UdpCommand::new(CommandIds::TimeCmd, PackageTypes::X50, 15);
    let command = Command::add_date_time(command);
    self.send(command)
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

impl Command {
  /// start_video requests the info (SPS/PPS) for video stream.
  ///
  /// # Examples
  /// ```no_run
  /// let mut drone = Command::new("192.168.10.1:8889");
  /// drone.connect(11111);
  /// // ...
  /// drone.start_video().unwrap();
  /// ```
  pub fn start_video(&mut self) -> Result {
    self.video.enabled = true;
    self.video.last_video_poll = SystemTime::now();
    self.send(UdpCommand::new_with_zero_sqn(CommandIds::VideoStartCmd, PackageTypes::X60, 0))
  }

  /// Set the video mode to 960x720 4:3 video, or 1280x720 16:9 zoomed video.
  /// 4:3 has a wider field of view (both vertically and horizontally), 16:9 is crisper.
  ///
  /// # Examples
  /// ```no_run
  /// let mut drone = Command::new("192.168.10.1:8889");
  /// drone.connect(11111);
  /// // ...
  /// drone.set_video_mode(VideoMode::M960x720).unwrap();
  /// ```
  pub fn set_video_mode(&mut self, mode: VideoMode) -> Result {
    self.video.mode = mode.clone();
    let mut cmd = UdpCommand::new_with_zero_sqn(CommandIds::VideoStartCmd, PackageTypes::X68, 1);
    cmd.write_u8(mode as u8);
    self.send(cmd)
  }

  /// Set the camera exposure level.
  /// param level: it can be 0, 1 or 2
  ///
  /// # Examples
  /// ```no_run
  /// let mut drone = Command::new("192.168.10.1:8889");
  /// drone.connect(11111);
  /// // ...
  /// drone.set_exposure(2).unwrap();
  /// ```
  pub fn set_exposure(&mut self, level: u8) -> Result {
    let mut cmd = UdpCommand::new(CommandIds::ExposureCmd, PackageTypes::X48, 1);
    cmd.write_u8(0);
    self.send(cmd)
  }

  /// set the video encoder rate for the camera.
  /// param rate: TODO: unknown
  ///
  /// # Examples
  /// ```no_run
  /// let mut drone = Command::new("192.168.10.1:8889");
  /// drone.connect(11111);
  /// // ...
  /// drone.get_video_bitrate(3).unwrap();
  /// ```
  pub fn get_video_bitrate(&mut self, rate: u8) -> Result {
    self.video.encoding_rate = rate.clone();
    let mut cmd = UdpCommand::new(CommandIds::VideoEncoderRateCmd, PackageTypes::X68, 1);
    cmd.write_u8(rate);
    self.send(cmd)
  }

  /// take a single picture and provide it to download it.
  ///
  /// # Examples
  /// ```no_run
  /// let mut drone = Command::new("192.168.10.1:8889");
  /// drone.connect(11111);
  /// // ...
  /// drone.take_picture(3).unwrap();
  ///
  /// @TODO: download image
  /// ```
  pub fn take_picture(&self) -> Result {
    self.send(UdpCommand::new(CommandIds::TakePictureCommand, PackageTypes::X68, 0))
  }
}

#[derive(Debug, Clone)]
pub struct UdpCommand {
  inner: Vec<u8>,
}

impl UdpCommand {
  pub fn new(cmd: CommandIds, pkt_type: PackageTypes, length: u16) -> UdpCommand {
    let mut cur = Cursor::new(Vec::new());
    cur.write_u8(START_OF_PACKET).expect("");
    cur.write_u16::<LittleEndian>((length + 11) << 3).expect("");
    cur.write_u8(crc8(cur.clone().into_inner())).expect("");
    cur.write_u8(pkt_type as u8).expect("");
    cur.write_u16::<LittleEndian>(cmd as u16).expect("");

    let nr = unsafe {
      let s = SEQ_NO.clone();
      SEQ_NO += 1;
      s
    };
    cur.write_u16::<LittleEndian>(nr).expect("");

    UdpCommand {
      inner: cur.into_inner(),
    }
  }
  pub fn new_with_zero_sqn(cmd: CommandIds, pkt_type: PackageTypes, length: u16) -> UdpCommand {
    let mut cur = Cursor::new(Vec::new());
    cur.write_u8(START_OF_PACKET).expect("");
    cur.write_u16::<LittleEndian>((length + 11) << 3).expect("");
    cur.write_u8(crc8(cur.clone().into_inner())).expect("");
    cur.write_u8(pkt_type as u8).expect("");
    cur.write_u16::<LittleEndian>(cmd as u16).expect("");
    cur.write_u16::<LittleEndian>(0).expect("");

    UdpCommand {
      inner: cur.into_inner(),
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
  fn into(mut self) -> Vec<u8> {
    self
      .inner
      .write_u16::<LittleEndian>(crc16(self.inner.clone()))
      .expect("");
    self.inner
  }
}

#[derive(Debug, Clone)]
pub struct Package {
  pub cmd: CommandIds,
  pub size: u16,
  pub sq_nr: u16,
  pub data: PackageData,
}

#[derive(Debug, Clone)]
pub enum Message {
  Data(Package),
  Response(ResponseMsg),
}

impl TryFrom<Vec<u8>> for Message {
  type Error = &'static str;

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
          let _crc16: u16 = (data.pop().unwrap() as u16) + ((data.pop().unwrap() as u16) << 8);
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
        let command = CommandIds::from(cur.read_u16::<LittleEndian>().unwrap().clone());
        return Ok(Message::Response(ResponseMsg::UnknownCommand(command)));
      }

      unsafe {
        println!("data len {:?}", data.len());
        let msg = String::from_utf8_unchecked(data.clone()[0..5].to_vec());
        println!("data {:?}", msg);
      }
      Err("invalid package")
    }
  }
}

#[derive(Debug, Clone)]
pub enum PackageData {
  FlightData(FlightData),
  WifiInfo(WifiInfo),
  LightInfo(LightInfo),
  Version(String),
  AtlInfo(u16),
  LogMessage(LogMessage),
  NoData(),
  Unknown(Vec<u8>),
}
