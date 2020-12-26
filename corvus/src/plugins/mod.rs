use crate::{config::ServiceTypeConfiguration, prelude::*, triggers::Triggers};
use async_trait::async_trait;
use bluetooth::BluetoothService;
use command::CommandService;

mod bluetooth;
mod command;
mod dht;

#[async_trait]
pub trait Plugin {
    async fn run(&self, name: String) -> Result<()>;
    async fn heartbeat(&self, name: String) -> Result<()>;
    async fn leader_heartbeat(&self, name: String, data: ClusterNodes) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum Plugins {
    Command {
        name:    String,
        trigger: Triggers,
        service: CommandService,
    },
    Bluetooth {
        name:    String,
        trigger: Triggers,
        service: BluetoothService,
    },
}

impl Plugins {
    pub fn new(config: Arc<ServiceConfiguration>, app: App) -> Self {
        let name = config.name.to_string();
        let trigger = Triggers::new(config.trigger.clone());
        match &*config.service {
            ServiceTypeConfiguration::Command { command, args } => Plugins::Command {
                name,
                trigger,
                service: CommandService::new(app, command.to_string(), args.clone()),
            },
            ServiceTypeConfiguration::Bluetooth { .. } => Plugins::Bluetooth {
                name,
                trigger,
                service: BluetoothService::new(app),
            },
        }
    }

    pub async fn heartbeat(&self) -> Result<()> {
        match self {
            Plugins::Command { service, name, .. } => service.heartbeat(name.to_string()).await,
            Plugins::Bluetooth { service, name, .. } => service.heartbeat(name.to_string()).await,
        }
    }

    pub async fn leader_heartbeat(&self, data: ClusterNodes) -> Result<()> {
        match self {
            Plugins::Command { service, name, .. } => {
                service.leader_heartbeat(name.to_string(), data).await
            }
            Plugins::Bluetooth { service, name, .. } => {
                service.leader_heartbeat(name.to_string(), data).await
            }
        }
    }

    pub async fn run(&self) -> Result<()> {
        match self {
            Plugins::Command { service, name, .. } => service.run(name.to_string()).await,
            Plugins::Bluetooth { service, name, .. } => service.run(name.to_string()).await,
        }
    }

    pub fn start(&self) -> Result<()> {
        let svc = self.clone();
        match self {
            Plugins::Command { trigger, .. } => trigger.init(svc),
            Plugins::Bluetooth { trigger, .. } => trigger.init(svc),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Plugins::Command { name, .. } => name,
            Plugins::Bluetooth { name, .. } => name,
        }
    }
}
