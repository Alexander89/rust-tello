use std::string::String;
use std::convert::TryInto;
use std::ops::Deref;

#[derive(Debug, Clone, Copy)]
pub struct MetaData {
    pitch: i16,
    roll: i16,
    yaw: i16,
    vgx: i16,
    vgy: i16,
    vgz: i16,
    templ: u16,
    temph: u16,
    tof: u16,
    h: u16,
    bat: u16,
    baro:f32,
    time: u16,
    agx:f32,
    agy:f32,
    agz:f32,
}

impl MetaData {
    fn new() -> MetaData {
        MetaData {
            pitch: 0,
            roll: 0,
            yaw: 0,
            vgx: 0,
            vgy: 0,
            vgz: 0,
            templ: 0,
            temph: 0,
            tof: 0,
            h: 0,
            bat: 0,
            baro: 0.0,
            time: 0,
            agx: 0.0,
            agy: 0.0,
            agz: 0.0,
        }
    }
}

macro_rules! extractValue {
    ($d:ident, $e:ident, $v:ident) => {
        if $d.starts_with(stringify!($e)) {
            let db = $d.get(stringify!($e).len() + 1..).unwrap();
            match db.parse() {
                Ok(value) => $v.$e = value, 
                Err(_) => ()
            }
            $v
        } else {
            $v
        }
    };
}

impl TryInto<MetaData> for String {
    type Error = ();

    fn try_into(self: String) -> Result<MetaData, Self::Error> {
        let meta_data: Vec<&str> = self.deref().split(';').collect();
        let mut data = MetaData::new();
        for d in meta_data.iter() {
            data = extractValue!(d, pitch, data);
            data = extractValue!(d, roll, data);
            data = extractValue!(d, yaw, data);
            data = extractValue!(d, vgx, data);
            data = extractValue!(d, vgy, data);
            data = extractValue!(d, vgz, data);
            data = extractValue!(d, templ, data);
            data = extractValue!(d, temph, data);
            data = extractValue!(d, tof, data);
            data = extractValue!(d, h, data);
            data = extractValue!(d, bat, data);
            data = extractValue!(d, baro, data);
            data = extractValue!(d, time, data);
            data = extractValue!(d, agx, data);
            data = extractValue!(d, agy, data);
            data = extractValue!(d, agz, data);
        }
        Ok(data)
    }
}