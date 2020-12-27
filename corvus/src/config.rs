use crate::*;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Configuration {
    pub node:    Arc<NodeConfiguration>,
    pub mqtt:    Arc<MQTTConfiguration>,
    #[serde(rename = "plugin")]
    pub plugins: Vec<Arc<PluginConfiguration>>,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            plugins: vec![Default::default()],
            node:    Default::default(),
            mqtt:    Default::default(),
        }
    }
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
    pub client_id:       String,
    pub host:            String,
    pub port:            u16,
    pub base_topic:      String,
    pub discovery_topic: String,
}

impl Default for MQTTConfiguration {
    fn default() -> Self {
        MQTTConfiguration {
            client_id:       "corvus".into(),
            host:            "localhost".into(),
            port:            1883,
            base_topic:      "corvus".into(),
            discovery_topic: "homeassistant".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields, rename_all = "snake_case", default)]
pub struct PluginConfiguration {
    pub name:    String,
    #[serde(rename = "definition")]
    pub plugin:  Arc<PluginOptions>,
    pub trigger: Arc<TriggerConfiguration>,
}

impl Default for PluginConfiguration {
    fn default() -> Self {
        Self {
            name:    Default::default(),
            trigger: Default::default(),
            plugin:  Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "snake_case")]
pub enum PluginOptions {
    Command {
        command: String,
        args:    Vec<String>,
    },
    Bluetooth {},
    DHT {
        device:  String,
        channel: u32,
    },
}

impl Default for PluginOptions {
    fn default() -> Self {
        Self::Bluetooth {}
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

    pub fn generate_default(file: PathBuf) -> Result<()> {
        let path = file
            .clone()
            .into_os_string()
            .into_string()
            .unwrap_or_default();
        let config = Self::default();
        let content = toml::to_vec(&config)?;
        let mut f = std::fs::File::create(file)
            .with_context(|| format!("Could not create file {}!", path))?;
        f.write_all(&content)?;
        Ok(())
    }
}
