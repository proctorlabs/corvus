use crate::{config::PluginOptions, prelude::*, triggers::Triggers};
use async_trait::async_trait;
use bluetooth::BluetoothPlugin;
use command::CommandPlugin;
use dht::DHTPlugin;

mod bluetooth;
mod command;
mod dht;

macro_rules! plugins {
    ($($name:ident $plugin:ty,)*) => {
        #[derive(Debug, Clone)]
        pub enum Plugins {
            $($name {
                name: String,
                trigger: Triggers,
                service: $plugin,
            },)*
        }

        impl Plugins {
            pub async fn heartbeat(&self) -> Result<()> {
                match self {
                    $(Plugins::$name { service, name, .. } => service.heartbeat(name.to_string()).await,)*
                }
            }

            pub async fn leader_heartbeat(&self, data: ClusterNodes) -> Result<()> {
                match self {
                    $(Plugins::$name { service, name, .. } => {
                        service.leader_heartbeat(name.to_string(), data).await
                    })*
                }
            }

            pub async fn run(&self) -> Result<()> {
                match self {
                    $(Plugins::$name { service, name, .. } => service.run(name.to_string()).await,)*
                }
            }

            pub fn start(&self) -> Result<()> {
                let svc = self.clone();
                match self {
                    $(Plugins::$name { trigger, .. } => trigger.init(svc),)*
                }
            }

            pub fn name(&self) -> &str {
                match self {
                    $(Plugins::$name { name, .. } => name,)*
                }
            }
        }
    };
}

plugins! {
    Command CommandPlugin,
    Bluetooth BluetoothPlugin,
    DHT DHTPlugin,
}

#[async_trait]
pub trait Plugin {
    async fn run(&self, name: String) -> Result<()>;
    async fn heartbeat(&self, name: String) -> Result<()>;
    async fn leader_heartbeat(&self, name: String, data: ClusterNodes) -> Result<()>;
}

impl Plugins {
    pub fn new(config: Arc<PluginConfiguration>, app: App) -> Self {
        let name = config.name.to_string();
        let trigger = Triggers::new(config.trigger.clone());
        match &*config.plugin {
            PluginOptions::Command { command, args } => Plugins::Command {
                name,
                trigger,
                service: CommandPlugin::new(
                    app.mqtt.clone(),
                    app.device_registry.clone(),
                    command.to_string(),
                    args.clone(),
                ),
            },
            PluginOptions::Bluetooth { .. } => Plugins::Bluetooth {
                name,
                trigger,
                service: BluetoothPlugin::new(app.device_registry.clone(), app.mqtt.clone()),
            },
            PluginOptions::DHT { device, channel } => Plugins::DHT {
                name: name.to_string(),
                trigger,
                service: DHTPlugin::new(
                    name,
                    app.mqtt.clone(),
                    app.device_registry.clone(),
                    device.into(),
                    *channel,
                ),
            },
        }
    }
}
