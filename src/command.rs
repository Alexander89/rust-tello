use std::net::{SocketAddr, UdpSocket};
use std::io::Cursor;
use byteorder::LittleEndian;
use byteorder::WriteBytesExt;
use std::ops::Shr;

pub struct Command {
    socket: UdpSocket,
}

pub const START_OF_PACKET: u8 = 0xcc;
pub const SEQUENCE_NUMBER: u16 = 0x01e4;

#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum CommandIds {
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

#[repr(u8)]
pub enum PackageTypes {
    Normal = 0x68,
    ExpThrowFileCompl = 0x48,
    Data = 0x50,
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

    pub fn poll(&self) {
        let mut meta_buf = [0; 1440];

        if let Ok(received) = self.socket.recv(&mut meta_buf) {
            let meta_data_str = meta_buf[..received].to_vec();
            println!("{:?}", meta_data_str);
        }

    }

}


pub struct UdpCommand{
    inner: Vec<u8>
};

impl UdpCommand {
    pub fn new(cmd: CommandIds, pkt_type: u8) -> UdpCommand {
        UdpCommand {
            inner: vec![
                START_OF_PACKET,
                0, 0,
                0,
                pkt_type,
                ((cmd as u8) & 0xff), (((cmd as u16) >> 8) & 0xff) as u8,
                0, 0
                ]
        }
    }

    pub fn pack_command(size: usize) {
        let mut _cur = Cursor::new(&mut Vec::<u8>::new());
        // buf = self.get_buffer()
        // if buf[0] == START_OF_PACKET:
        //     buf[1], buf[2] = le16(len(buf)+2)
        //     buf[1] = (buf[1] << 3)
        //     buf[3] = crc.crc8(buf[0:3])
        //     buf[7], buf[8] = le16(seq_num)
        //     self.add_int16(crc.crc16(buf))
    }
}

impl Into<Vec<u8>> for UdpCommand {
    fn into(self) -> Vec<u8> {
        let mut data = {
            let length = self.inner.len() as u16;
            let mut cur = Cursor::new(&mut self.inner);

            cur.set_position(1);
            cur.write_u16::<LittleEndian>(length);
            cur.set_position(3);
            crc8::Crc8::create_msb()calc()

            cur.into_inner()
        };
        data[1] = data[1] << 3;
        data.to_vec()
    }
}