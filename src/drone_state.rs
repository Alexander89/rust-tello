use super::PackageData;

///
/// Represents the last received meta data from the drone
///
#[derive(Debug, Clone, Default)]
pub struct DroneMeta {
    flight: Option<FlightData>,
    wifi: Option<WifiInfo>,
    light: Option<LightInfo>,
}

impl DroneMeta {
    pub fn get_flight_data(&self) -> Option<FlightData> {
        self.flight.clone()
    }
    pub fn get_wifi_info(&self) -> Option<WifiInfo> {
        self.wifi.clone()
    }
    pub fn get_light_info(&self) -> Option<LightInfo> {
        self.light.clone()
    }
    /// applies the package to the current data.
    /// It ignore non Meta package data and just overwrite the current metadata
    pub fn update(&mut self, package: &PackageData) {
        match package {
            PackageData::FlightData(fd) => self.flight = Some(fd.clone()),
            PackageData::WifiInfo(wifi) => self.wifi = Some(wifi.clone()),
            PackageData::LightInfo(li) => self.light = Some(li.clone()),
            _ => (),
        };
    }
}

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{BufRead, Cursor, Seek, SeekFrom};

fn int16(val0: u8, val1: u8) -> i16 {
    if val1 != 0 {
        (((val0 as i32) | ((val1 as i32) << 8)) - 0x10000) as i16
    } else {
        (val0 as i16) | ((val1 as i16) << 8)
    }
}

#[derive(Clone)]
pub struct FlightData {
    pub height: i16,
    pub north_speed: i16,
    pub east_speed: i16,
    pub ground_speed: i16,
    pub fly_time: i16,
    pub imu_state: bool,
    pub pressure_state: bool,
    pub down_visual_state: bool,
    pub power_state: bool,
    pub battery_state: bool,
    pub gravity_state: bool,
    pub wind_state: bool,
    pub imu_calibration_state: u8,
    pub battery_percentage: u8,
    pub drone_battery_left: i16,
    pub drone_fly_time_left: i16,

    pub em_sky: bool,
    pub em_ground: bool,
    pub em_open: bool,
    pub drone_hover: bool,
    pub outage_recording: bool,
    pub battery_low: bool,
    pub battery_lower: bool,
    pub factory_mode: bool,

    pub fly_mode: u8,
    pub throw_fly_timer: u8,
    pub camera_state: u8,
    pub electrical_machinery_state: u8,
    pub front_in: bool,
    pub front_out: bool,
    pub front_lsc: bool,
    pub temperature_height: bool,
}

impl std::fmt::Debug for FlightData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FlightData {{ alt: {}, north_sp: {}, east_sp: {}, ground_sp: {}, fly_time: {}, imu_cal: {}, battery: {}, battery_left: {}, fly_time_left: {}, fly_mode: {}, throw_fly_timer: {}, camera: {}, em: {}}}",
            self.height, self.north_speed, self.east_speed, self.ground_speed, self.fly_time, self.imu_calibration_state, self.battery_percentage,
            self.drone_battery_left, self.drone_fly_time_left, self.fly_mode, self.throw_fly_timer, self.camera_state, self.electrical_machinery_state
        )
    }
}

impl From<Vec<u8>> for FlightData {
    fn from(data: Vec<u8>) -> FlightData {
        FlightData {
            height: int16(data[0], data[1]),
            north_speed: int16(data[2], data[3]),
            east_speed: int16(data[4], data[5]),
            ground_speed: int16(data[6], data[7]),
            fly_time: int16(data[8], data[9]),

            imu_state: ((data[10]) & 0x1) != 0,
            pressure_state: ((data[10] >> 1) & 0x1) != 0,
            down_visual_state: ((data[10] >> 2) & 0x1) != 0,
            power_state: ((data[10] >> 3) & 0x1) != 0,
            battery_state: ((data[10] >> 4) & 0x1) != 0,
            gravity_state: ((data[10] >> 5) & 0x1) != 0,
            wind_state: ((data[10] >> 7) & 0x1) != 0,

            imu_calibration_state: data[11],
            battery_percentage: data[12],
            drone_battery_left: int16(data[13], data[14]),
            drone_fly_time_left: int16(data[15], data[16]),

            em_sky: ((data[17]) & 0x1) != 0,
            em_ground: ((data[17] >> 1) & 0x1) != 0,
            em_open: ((data[17] >> 2) & 0x1) != 0,
            drone_hover: ((data[17] >> 3) & 0x1) != 0,
            outage_recording: ((data[17] >> 4) & 0x1) != 0,
            battery_low: ((data[17] >> 5) & 0x1) != 0,
            battery_lower: ((data[17] >> 6) & 0x1) != 0,
            factory_mode: ((data[17] >> 7) & 0x1) != 0,

            fly_mode: data[18],
            throw_fly_timer: data[19],
            camera_state: data[20],
            electrical_machinery_state: data[21],

            front_in: ((data[22]) & 0x1) != 0,
            front_out: ((data[22] >> 1) & 0x1) != 0,
            front_lsc: ((data[22] >> 2) & 0x1) != 0,

            temperature_height: ((data[23]) & 0x1) != 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WifiInfo {
    strength: u8,
    disturb: u8,
}
impl From<Vec<u8>> for WifiInfo {
    fn from(data: Vec<u8>) -> WifiInfo {
        WifiInfo {
            strength: data[0],
            disturb: data[1],
        }
    }
}

#[derive(Debug, Clone)]
pub struct LightInfo {
    good: u8,
}
impl From<Vec<u8>> for LightInfo {
    fn from(data: Vec<u8>) -> LightInfo {
        LightInfo { good: data[0] }
    }
}

#[derive(Debug, Clone)]
pub struct LogMessage {
    pub id: u16,
    pub message: String,
}
impl From<Vec<u8>> for LogMessage {
    fn from(data: Vec<u8>) -> LogMessage {
        let mut cur = Cursor::new(data);
        cur.seek(SeekFrom::Start(9)).unwrap();
        let id: u16 = cur.read_u16::<LittleEndian>().unwrap();
        let mut msg: Vec<u8> = Vec::new();
        cur.read_until(0, &mut msg).unwrap();
        LogMessage {
            id,
            message: String::from_utf8(msg).unwrap(),
        }
    }
}
