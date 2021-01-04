use super::HassDiscoveryPayload;
use crate::prelude::{constants::*, *};

#[derive(Debug, Clone, Deref)]
pub struct Device(Arc<DeviceData>);

#[derive(Debug)]
pub struct DeviceData {
    id:                  String,
    display_name:        String,
    typ:                 DeviceType,
    cluster_wide:        bool,
    location:            String,
    plugin:              String,
    base_topic:          String,
    unit_of_measurement: Option<String>,
}

pub fn clean_name(s: &str) -> String {
    s.to_lowercase()
        .replace(":", "")
        .replace("-", "_")
        .replace(" ", "_")
}

impl DeviceData {
    pub fn new(
        display_name: String,
        typ: DeviceType,
        location: String,
        base_topic: String,
        plugin: String,
    ) -> Self {
        Self {
            id: clean_name(&display_name),
            unit_of_measurement: None,
            cluster_wide: false,
            plugin,
            display_name,
            typ,
            location,
            base_topic,
        }
    }

    pub fn with_unit_of_measurement(mut self, unit: String) -> Self {
        self.unit_of_measurement = Some(unit);
        self
    }

    pub fn into_cluster_device(mut self) -> Self {
        self.cluster_wide = true;
        self
    }

    pub fn build(self) -> Device {
        Device(Arc::new(self))
    }
}

impl Device {
    #[allow(clippy::field_reassign_with_default)]
    pub fn to_discovery(&self) -> HassDiscoveryPayload {
        let mut mfr = HassDeviceInformation::default();
        mfr.name = Some(self.location.to_string());
        mfr.model = Some(crate_name!().into());
        mfr.manufacturer = Some(crate_authors!().into());
        mfr.sw_version = Some(crate_version!().into());
        mfr.identifiers = Some(self.uniq_id());

        let mut ent = HassDiscoveryPayload::default();
        ent.device = Some(mfr);
        ent.name = Some(self.display_name().into());
        ent.icon = Some(self.icon().into());
        ent.unique_id = Some(self.uniq_id());
        ent.base_topic = Some(self.device_base());
        ent.device_class = self.device_class();
        ent.state_topic = Some("~stat".to_string());
        ent.json_attributes_topic = Some("~attr".to_string());
        ent.payload_available = Some("online".into());
        ent.payload_not_available = Some("offline".into());
        ent.unit_of_measurement = self.unit_of_measurement.clone();

        if !self.cluster_wide() {
            ent.availability_topic = Some(self.avty_topic());
        }

        ent
    }

    // We return references here when possible, caller can clone if needed
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn plugin(&self) -> &str {
        &self.plugin
    }

    pub fn device_type(&self) -> String {
        self.typ.to_string()
    }

    pub fn uniq_id(&self) -> String {
        if self.cluster_wide() {
            self.id().into()
        } else {
            format!("{}_{}", clean_name(&self.location), self.id())
        }
    }

    fn node_base(&self) -> String {
        if self.cluster_wide() {
            format!("{}/cluster/", self.base_topic)
        } else {
            format!("{}/nodes/{}/", self.base_topic, clean_name(&self.location))
        }
    }

    fn device_base(&self) -> String {
        format!("{}{}/", self.node_base(), self.uniq_id())
    }

    pub fn avty_topic(&self) -> String {
        format!("{}avty", self.node_base())
    }

    pub fn stat_topic(&self) -> String {
        format!("{}stat", self.device_base())
    }

    pub fn attr_topic(&self) -> String {
        format!("{}attr", self.device_base())
    }

    pub fn device_class(&self) -> Option<String> {
        match &self.typ {
            DeviceType::Sensor(SensorDeviceClass::None)
            | DeviceType::BinarySensor(BinarySensorDeviceClass::None) => None,
            DeviceType::Sensor(dc) => Some(dc.to_string()),
            DeviceType::BinarySensor(dc) => Some(dc.to_string()),
            _ => None,
        }
    }

    pub fn icon(&self) -> &'static str {
        self.typ.icon()
    }

    pub fn cluster_wide(&self) -> bool {
        self.cluster_wide
    }
}

#[derive(Clone, Debug, Display)]
pub enum DeviceType {
    #[display(fmt = "sensor")]
    Sensor(SensorDeviceClass),
    #[display(fmt = "binary_sensor")]
    BinarySensor(BinarySensorDeviceClass),
    #[display(fmt = "media_player")]
    MediaPlayer,
    #[display(fmt = "switch")]
    Switch,
    #[display(fmt = "light")]
    Light,
    #[display(fmt = "thermostat")]
    Thermostat,
}

#[derive(Clone, Debug, Display)]
pub enum SensorDeviceClass {
    #[display(fmt = "none")]
    None,
    #[display(fmt = "battery")]
    Battery,
    #[display(fmt = "humidity")]
    Humidity,
    #[display(fmt = "illuminance")]
    Illuminance,
    #[display(fmt = "signal_strength")]
    SignalStrength,
    #[display(fmt = "temperature")]
    Temperature,
    #[display(fmt = "power")]
    Power,
    #[display(fmt = "pressure")]
    Pressure,
    #[display(fmt = "timestamp")]
    Timestamp,
    #[display(fmt = "current")]
    Current,
    #[display(fmt = "energy")]
    Energy,
    #[display(fmt = "power_factor")]
    PowerFactor,
    #[display(fmt = "voltage")]
    Voltage,
}

#[derive(Clone, Debug, Display)]
pub enum BinarySensorDeviceClass {
    #[display(fmt = "none")]
    None,
    #[display(fmt = "battery")]
    Battery, // on means low, off means normal
    #[display(fmt = "battery_charging")]
    BatteryCharging, // on means charging, off means not charging
    #[display(fmt = "cold")]
    Cold, // on means cold, off means normal
    #[display(fmt = "connectivity")]
    Connectivity, // on means connected, off means disconnected
    #[display(fmt = "door")]
    Door, // on means open, off means closed
    #[display(fmt = "garage_door")]
    GarageDoor, // on means open, off means closed
    #[display(fmt = "gas")]
    Gas, // on means gas detected, off means no gas (clear)
    #[display(fmt = "heat")]
    Heat, // on means hot, off means normal
    #[display(fmt = "light")]
    Light, // on means light detected, off means no light
    #[display(fmt = "lock")]
    Lock, // on means open (unlocked), off means closed (locked)
    #[display(fmt = "moisture")]
    Moisture, // on means moisture detected (wet), off means no moisture (dry)
    #[display(fmt = "motion")]
    Motion, // on means motion detected, off means no motion (clear)
    #[display(fmt = "moving")]
    Moving, // on means moving, off means not moving (stopped)
    #[display(fmt = "occupancy")]
    Occupancy, // on means occupied, off means not occupied (clear)
    #[display(fmt = "opening")]
    Opening, // on means open, off means closed
    #[display(fmt = "plug")]
    Plug, // on means device is plugged in, off means device is unplugged
    #[display(fmt = "power")]
    Power, // on means power detected, off means no power
    #[display(fmt = "presence")]
    Presence, // on means home, off means away
    #[display(fmt = "problem")]
    Problem, // on means problem detected, off means no problem (OK)
    #[display(fmt = "safety")]
    Safety, // on means unsafe, off means safe
    #[display(fmt = "smoke")]
    Smoke, // on means smoke detected, off means no smoke (clear)
    #[display(fmt = "sound")]
    Sound, // on means sound detected, off means no sound (clear)
    #[display(fmt = "vibration")]
    Vibration, // on means vibration detected, off means no vibration (clear)
    #[display(fmt = "window")]
    Window, // on means open, off means closed
}

impl DeviceType {
    pub fn icon(&self) -> &'static str {
        match self {
            DeviceType::Thermostat => HassIcons::THERMOMETER,
            DeviceType::Light => HassIcons::LIGHT,
            DeviceType::Switch => HassIcons::POWER,
            DeviceType::MediaPlayer => HassIcons::TELEVISION,
            DeviceType::Sensor(c) => c.icon(),
            DeviceType::BinarySensor(c) => c.icon(),
        }
    }

    pub fn device_class(&self) -> Option<String> {
        match self {
            DeviceType::Sensor(c) => Some(c.to_string()),
            DeviceType::BinarySensor(c) => Some(c.to_string()),
            _ => None,
        }
    }
}

impl SensorDeviceClass {
    pub fn icon(&self) -> &'static str {
        match self {
            SensorDeviceClass::Battery => HassIcons::EYE,
            SensorDeviceClass::Humidity => HassIcons::WATER_PERCENT,
            SensorDeviceClass::Illuminance => HassIcons::EYE,
            SensorDeviceClass::SignalStrength => HassIcons::BLUETOOTH_WAVE,
            SensorDeviceClass::Temperature => HassIcons::THERMOMETER,
            SensorDeviceClass::Power => HassIcons::POWER,
            SensorDeviceClass::Pressure => HassIcons::EYE,
            SensorDeviceClass::Timestamp => HassIcons::EYE,
            SensorDeviceClass::Current => HassIcons::FLASH,
            SensorDeviceClass::Energy => HassIcons::FLASH,
            SensorDeviceClass::PowerFactor => HassIcons::FLASH,
            SensorDeviceClass::Voltage => HassIcons::FLASH,
            SensorDeviceClass::None => HassIcons::EYE,
        }
    }
}

impl BinarySensorDeviceClass {
    pub fn icon(&self) -> &'static str {
        match self {
            BinarySensorDeviceClass::None => HassIcons::POWER,
            BinarySensorDeviceClass::Battery => HassIcons::EYE,
            BinarySensorDeviceClass::BatteryCharging => HassIcons::EYE,
            BinarySensorDeviceClass::Cold => HassIcons::EYE,
            BinarySensorDeviceClass::Connectivity => HassIcons::EYE,
            BinarySensorDeviceClass::Door => HassIcons::SQUARE,
            BinarySensorDeviceClass::GarageDoor => HassIcons::GARAGE,
            BinarySensorDeviceClass::Gas => HassIcons::EYE,
            BinarySensorDeviceClass::Heat => HassIcons::EYE,
            BinarySensorDeviceClass::Light => HassIcons::EYE,
            BinarySensorDeviceClass::Lock => HassIcons::EYE,
            BinarySensorDeviceClass::Moisture => HassIcons::WATER_PERCENT,
            BinarySensorDeviceClass::Motion => HassIcons::EYE,
            BinarySensorDeviceClass::Moving => HassIcons::EYE,
            BinarySensorDeviceClass::Occupancy => HassIcons::EYE,
            BinarySensorDeviceClass::Opening => HassIcons::SQUARE,
            BinarySensorDeviceClass::Plug => HassIcons::EYE,
            BinarySensorDeviceClass::Power => HassIcons::FLASH,
            BinarySensorDeviceClass::Presence => HassIcons::EYE,
            BinarySensorDeviceClass::Problem => HassIcons::EYE,
            BinarySensorDeviceClass::Safety => HassIcons::EYE,
            BinarySensorDeviceClass::Smoke => HassIcons::EYE,
            BinarySensorDeviceClass::Sound => HassIcons::EYE,
            BinarySensorDeviceClass::Vibration => HassIcons::EYE,
            BinarySensorDeviceClass::Window => HassIcons::EYE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceUpdate {
    pub device: Option<Device>,
    pub value:  Document,
    pub attr:   Document,
}
