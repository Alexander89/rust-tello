

fn int16(val0: u8, val1: u8) -> i16  {
    if (val1 & 0xff) != 0 {
        (((val0 & 0xff) as i16) | (((val1 & 0xff) as i16) << 8))// - 0x10000
    }
    else {
        ((val0 & 0xff) as i16) | (((val1 & 0xff) as i16) << 8)
    }
}

#[derive(Debug, Clone)]
pub struct FlightData {
    height: i16,
    north_speed: i16,
    east_speed: i16,
    ground_speed: i16,
    fly_time: i16,
    imu_state: bool,
    pressure_state: bool,
    down_visual_state: bool,
    power_state: bool,
    battery_state: bool,
    gravity_state: bool,
    wind_state: bool,
    imu_calibration_state: u8,
    battery_percentage: u8,
    drone_battery_left: i16,
    drone_fly_time_left: i16,

    em_sky: bool,
    em_ground: bool,
    em_open: bool,
    drone_hover: bool,
    outage_recording: bool,
    battery_low: bool,
    battery_lower: bool,
    factory_mode: bool,

    fly_mode: u8,
    throw_fly_timer: u8,
    camera_state: u8,
    electrical_machinery_state: u8,
    front_in: bool,
    front_out: bool,
    front_lsc: bool,
    temperature_height: bool,
}
impl From<Vec<u8>> for FlightData {
    fn from(data: Vec<u8>) -> FlightData {
        FlightData {
            height: int16(data[0], data[1]),
            north_speed: int16(data[2], data[3]),
            east_speed: int16(data[4], data[5]),
            ground_speed: int16(data[6], data[7]),
            fly_time: int16(data[8], data[9]),
            
            imu_state: ((data[10] >> 0) & 0x1) != 0,
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
            
            em_sky: ((data[17] >> 0) & 0x1) != 0,
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
            
            front_in: ((data[22] >> 0) & 0x1) != 0,
            front_out: ((data[22] >> 1) & 0x1) != 0,
            front_lsc: ((data[22] >> 2) & 0x1) != 0,
            
            temperature_height: ((data[23] >> 0) & 0x1) != 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WifiInfo {
    strength: u8,
    disturb: u8
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
        LightInfo {
            good: data[0]
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogMessage {
    message: String,
}
impl From<Vec<u8>> for LogMessage {
    fn from(data: Vec<u8>) -> LogMessage {
        LogMessage {
            message: String::from_utf8(data).unwrap()
        }
    }
}