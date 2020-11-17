use crate::*;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Configuration {
    pub node: Arc<NodeConfiguration>,
    pub mqtt: Arc<MQTTConfiguration>,
    #[serde(alias = "service")]
    pub services: Vec<Arc<ServiceConfiguration>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct NodeConfiguration {
    pub location: String,
}

impl Default for NodeConfiguration {
    fn default() -> Self {
        NodeConfiguration {
            location: "home".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields, rename_all = "snake_case", default)]
pub struct MQTTConfiguration {
    pub client_id: String,
    pub host: String,
    pub port: u16,
    pub base_topic: String,
    pub discovery_topic: String,
}

impl Default for MQTTConfiguration {
    fn default() -> Self {
        MQTTConfiguration {
            client_id: "corvus".into(),
            host: "localhost".into(),
            port: 1883,
            base_topic: "corvus".into(),
            discovery_topic: "homeassistant".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields, rename_all = "snake_case", default)]
pub struct ServiceConfiguration {
    pub name: String,
    pub service: Arc<ServiceTypeConfiguration>,
    pub trigger: Arc<TriggerConfiguration>,
}

impl Default for ServiceConfiguration {
    fn default() -> Self {
        ServiceConfiguration {
            name: Default::default(),
            trigger: Default::default(),
            service: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "snake_case")]
pub enum ServiceTypeConfiguration {
    Command {
        command: String,
        args: Vec<String>,
    },
    Bluetooth {},
}

impl Default for ServiceTypeConfiguration {
    fn default() -> Self {
        ServiceTypeConfiguration::Command {
            command: Default::default(),
            args: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields, untagged, rename_all = "snake_case")]
pub enum TriggerConfiguration {
    Start { on_start: bool },
    Interval { interval: u64 },
    MQTT { mqtt_topic: String },
}

impl Default for TriggerConfiguration {
    fn default() -> Self {
        TriggerConfiguration::Start { on_start: true }
    }
}

impl Configuration {
    pub fn load(file: PathBuf) -> Result<Arc<Self>> {
        let path = file
            .clone()
            .into_os_string()
            .into_string()
            .unwrap_or_default();
        let mut f = std::fs::File::open(file)
            .with_context(|| format!("Could not load configuration from file {}!", path))?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)?;
        let c = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse contents of {}!", path))?;
        Ok(Arc::new(c))
    }
}
