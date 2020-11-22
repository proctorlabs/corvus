use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub typ:  DeviceType,
}

#[derive(Clone, Debug, Display)]
pub enum DeviceType {
    #[display(fmt = "sensor")]
    Sensor,
    #[display(fmt = "binary_sensor")]
    BinarySensor,
    #[display(fmt = "media_player")]
    MediaPlayer,
    #[display(fmt = "switch")]
    Switch,
    #[display(fmt = "light")]
    Light,
    #[display(fmt = "thermostat")]
    Thermostat,
}

#[derive(Debug, Clone)]
pub struct DeviceUpdate {
    pub name:  String,
    pub value: Document,
    pub attr:  Option<Document>,
}
