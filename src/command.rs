use std::net::{SocketAddr, UdpSocket};
use std::io::{Cursor, Read, Seek, SeekFrom};
use byteorder::{WriteBytesExt, ReadBytesExt, LittleEndian};
use chrono::prelude::*;
use crate::crc::{crc8, crc16};
use crate::drone_messages::{FlightData, WifiInfo, LightInfo, LogMessage};
use std::convert::TryFrom;


static mut SEQ_NO: u16 = 0;

type Result = std::result::Result<(), ()>;

pub struct Command {
    socket: UdpSocket,
}

pub const START_OF_PACKET: u8 = 0xcc;

#[derive(Debug, Clone, Copy)]
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
    SetAltLimitCmd = 0x0058,
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
    fn from(value:u16) -> CommandIds {
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
            0x0058 => CommandIds::SetAltLimitCmd,
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
            _ => CommandIds::Undefined
        }
    }
}

#[derive(Debug, Clone)]
pub enum ResponseMsg {
    Connected(String),
    UnknownCommand(CommandIds),
    Unknown,
}

#[repr(u8)]
pub enum PackageTypes {
    X48ExpThrowFileCompl = 0x48,
    X50Data = 0x50,
    X60NoSqNo = 0x60,
    X68Normal = 0x68,
}

//Flip commands taken from Go version of code
pub enum Flip {
    //flips forward.
    Front = 0,
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

impl Command {
    pub fn new(ip: &str) -> Command {
        let bind_addr = SocketAddr::from(([0, 0, 0, 0], 8889));
        let socket = UdpSocket::bind(&bind_addr).expect("couldn't bind to command address");
        socket.set_nonblocking(true).unwrap();
        socket.connect(ip).expect("connect command socket failed"); 
        
        Command {
            socket
        }
    }

    pub fn connect(&self, video_port: u16) -> usize {
        let mut data = b"conn_req:  ".to_vec();
        let mut cur = Cursor::new(&mut data);
        cur.set_position(9);
        cur.write_u16::<LittleEndian>(video_port).unwrap();
        println!("connect command {:?}", data);
        self.socket.send(&data).expect("network should be usable")
    }

    pub fn send(&self, command: UdpCommand) -> Result {
        let data: Vec<u8> = command.into();
        if self.socket.send(&data).is_ok() {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn poll(&self) {
        let mut meta_buf = [0; 1440];

        if let Ok(received) = self.socket.recv(&mut meta_buf) {
            let data = meta_buf[..received].to_vec();
            match Message::try_from(data) {
                Ok(Message::data(d)) => println!("Data: {:?}", d),
                Ok(Message::response(r)) => println!("Response : {:?}", r),
                Err(e) => println!("Error {:?}", e),
                _ => ()
            }
        }

    }

}

impl Command {
    pub fn take_off(&self) -> Result {
        self.send(UdpCommand::new(CommandIds::TakeoffCmd, PackageTypes::X68Normal, 0))
    }    
    pub fn land(&self) -> Result {
        let mut command = UdpCommand::new(CommandIds::LandCmd, PackageTypes::X68Normal, 1);
        command.write_u8(0x00);
        self.send(command)
    }  
    pub fn stop_land(&self) -> Result {
        let mut command = UdpCommand::new(CommandIds::LandCmd, PackageTypes::X68Normal, 1);
        command.write_u8(0x01);
        self.send(command)
    }
    pub fn start_video(&self) -> Result {
        self.send(UdpCommand::new_with_zero_sqn(CommandIds::VideoStartCmd, PackageTypes::X60NoSqNo, 0))
    }    
    // SendDateTime sends the current date/time to the drone.
    pub fn send_date_time(&self) -> Result {
        let mut command = UdpCommand::new(CommandIds::TimeCmd, PackageTypes::X50Data, 1);
        let now = Local::now();
        command.write_u8(0x00);
        command.write_u16(now.hour() as u16);
        command.write_u16(now.minute() as u16);
        command.write_u16(now.second() as u16);
        command.write_u16((now.nanosecond() >> 16) as u16 );
        command.write_u16((now.nanosecond() & 0xffff) as u16 );

        let d: Vec<u8> = command.clone().into();
        self.send(command)
    }
}

#[derive(Debug, Clone)]
pub struct UdpCommand{
    inner: Vec<u8>
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
            inner: cur.into_inner()
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
            inner: cur.into_inner()
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
}

impl Into<Vec<u8>> for UdpCommand {
    fn into(self) -> Vec<u8> {
        let mut cur = Cursor::new(self.inner);
        cur.seek(SeekFrom::End(0)).unwrap();
        cur.write_u16::<LittleEndian>(crc16(cur.clone().into_inner())).expect("");
        cur.into_inner()
    }
}

#[derive(Debug, Clone)]
pub struct Package {
    cmd: CommandIds,
    size: u16,
    sq_nr: u16,
    data: PackageData,
}

enum Message {
    data(Package),
    response(ResponseMsg)
}

impl TryFrom<Vec<u8>> for Message {
    type Error = &'static str;

    fn try_from(data: Vec<u8>) -> std::result::Result<Self, Self::Error>  {
        let mut cur = Cursor::new(data);
        if let Ok(START_OF_PACKET) = cur.read_u8() {
            let size = (cur.read_u16::<LittleEndian>().unwrap() >> 3) - 11;
            let _crc8 = cur.read_u8().unwrap();
            let _pkt_type = cur.read_u8().unwrap();
            let cmd = CommandIds::from(cur.read_u16::<LittleEndian>().unwrap());
            let sq_nr = cur.read_u16::<LittleEndian>().unwrap();
            let data = if size > 0 {
                let mut data : Vec<u8>= Vec::with_capacity(size as usize);
                cur.read_to_end(&mut data).unwrap();
                match cmd {
                    CommandIds::FlightMsg => PackageData::FlightData(FlightData::from(data)),
                    CommandIds::WifiMsg => PackageData::WifiInfo(WifiInfo::from(data)),
                    CommandIds::LightMsg => PackageData::LightInfo(LightInfo::from(data)),
                    //CommandIds::LogHeaderMsg => PackageData::LogMessage(LogMessage::from(data)),
                    _ => PackageData::Unknown(),
                }
            } else {
                PackageData::NoData()
            };

            Ok(Message::data(Package {
                cmd,
                size,
                sq_nr,
                data,
            }))

        } else {
            let data = cur.into_inner();
            if data[0..9].to_vec() == b"conn_ack:" {
                return Ok(Message::response(ResponseMsg::Connected(String::from_utf8(data).unwrap())))
            } else if data[0..16].to_vec() == b"unknown command:" {
                println!("data {:?}", data[17..].to_vec());
                let mut cur = Cursor::new(data[17..].to_owned());
                let command = CommandIds::from(cur.read_u16::<LittleEndian>().unwrap().clone());
                return Ok(Message::response(ResponseMsg::UnknownCommand(command)))
            }
            
            unsafe {
                println!("data len {:?}", data.len());
                let msg = String::from_utf8_unchecked(data.clone()[0..16].to_vec());
                println!("data {:?}", msg);
            }
            Err("invalid package")
        }
    }
    
}

#[derive(Debug, Clone)]
enum PackageData {
    FlightData(FlightData),
    WifiInfo(WifiInfo),
    LightInfo(LightInfo),
    LogMessage(LogMessage),
    Response(ResponseMsg),
    NoData(),
    Unknown(),
}