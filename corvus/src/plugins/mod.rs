use crate::{config::PluginOptions, prelude::*, triggers::Triggers};
use async_trait::async_trait;
use bluetooth::BluetoothPlugin;
use command::CommandPlugin;
use dht::DHTPlugin;
use std::collections::HashMap;

mod bluetooth;
mod command;
mod dht;

#[derive(Debug, Clone, Deref, Default)]
pub struct PluginManager(Arc<Mutex<HashMap<String, Plugins>>>);

impl PluginManager {
    pub async fn process_update(&self, plugin: &str, data: Document) -> Result<()> {
        let p = self.lock().await.get(plugin).cloned();
        if let Some(plugin) = p {
            plugin.process_update(data).await?;
        }
        Ok(())
    }

    pub async fn init_plugins(&self, config: &Configuration, app: &App) -> Result<()> {
        let mut svcs = self.lock().await;
        for svc in config.plugins.iter() {
            let s = Plugins::new(svc.clone(), app.clone());
            s.start()?;
            svcs.insert(s.name().into(), s);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deref)]
pub struct Plugins(Arc<PluginData>);

macro_rules! plugins {
    ($($name:ident $plugin:ty,)*) => {
        #[derive(Debug)]
        pub enum PluginData {
            $($name {
                name: String,
                trigger: Triggers,
                service: $plugin,
            },)*
        }

        impl Plugins {
            pub async fn heartbeat(&self) -> Result<()> {
                match &***self {
                    $(PluginData::$name { service, name, .. } => service.heartbeat(name.to_string()).await,)*
                }
            }

            pub async fn leader_heartbeat(&self, data: ClusterNodes) -> Result<()> {
                match &***self {
                    $(PluginData::$name { service, name, .. } => {
                        service.leader_heartbeat(name.to_string(), data).await
                    })*
                }
            }

            pub async fn run(&self) -> Result<()> {
                match &***self {
                    $(PluginData::$name { service, name, .. } => service.run(name.to_string()).await,)*
                }
            }

            pub fn start(&self) -> Result<()> {
                let svc = self.clone();
                match &***self {
                    $(PluginData::$name { trigger, .. } => trigger.init(svc),)*
                }
            }

            pub async fn process_update(&self, data: Document) -> Result<()> {
                match &***self {
                    $(PluginData::$name { service, .. } => service.process_update(data).await,)*
                }
            }

            pub fn name(&self) -> &str {
                match &***self {
                    $(PluginData::$name { name, .. } => name,)*
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
    async fn process_update(&self, _: Document) -> Result<()> {
        Ok(())
    }
}

impl Plugins {
    pub fn new(config: Arc<PluginConfiguration>, app: App) -> Self {
        let name = config.name.to_string();
        let trigger = Triggers::new(config.trigger.clone());
        match &*config.plugin {
            PluginOptions::Command { command, args } => Plugins(Arc::new(PluginData::Command {
                name,
                trigger,
                service: CommandPlugin::new(
                    app.mqtt.clone(),
                    app.device_registry.clone(),
                    command.to_string(),
                    args.clone(),
                ),
            })),
            PluginOptions::Bluetooth { .. } => Plugins(Arc::new(PluginData::Bluetooth {
                name: name.to_string(),
                trigger,
                service: BluetoothPlugin::new(
                    name,
                    app.config.node.location.to_string(),
                    app.device_registry.clone(),
                    app.mqtt.clone(),
                ),
            })),
            PluginOptions::DHT { device, channel } => Plugins(Arc::new(PluginData::DHT {
                name: name.to_string(),
                trigger,
                service: DHTPlugin::new(
                    name,
                    app.mqtt.clone(),
                    app.device_registry.clone(),
                    device.into(),
                    *channel,
                ),
            })),
        }
    }
}
