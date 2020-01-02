use crate::drone_messages::{FlightData, WifiInfo, LightInfo};
use crate::command::{PackageData};

#[derive(Debug, Clone)]
pub struct DroneState {
    flight: Option<FlightData>,
    wifi: Option<WifiInfo>,
    light: Option<LightInfo>,
}

impl DroneState {
    pub fn new() -> DroneState {
        DroneState {
            flight: None,
            wifi: None,
            light: None,
        }
    }

    pub fn update(&mut self, package: &PackageData) {
        match package {
            PackageData::FlightData(fd) => self.flight = Some(fd.clone()),
            PackageData::WifiInfo(wifi) => self.wifi = Some(wifi.clone()),
            PackageData::LightInfo(li) => self.light = Some(li.clone()),
            _ => ()
        };
    }
}