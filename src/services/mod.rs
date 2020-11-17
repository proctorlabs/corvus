use {
    crate::{config::ServiceTypeConfiguration, prelude::*, triggers::Triggers},
    bluetooth::BluetoothService,
    command::CommandService,
    async_trait::async_trait,
};

mod bluetooth;
mod command;

#[async_trait]
pub trait Service {
    async fn run(&self, name: String) -> Result<()>;
    fn device_type(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub enum Services {
    Command {
        name: String,
        trigger: Triggers,
        service: CommandService,
    },
    Bluetooth {
        name: String,
        trigger: Triggers,
        service: BluetoothService,
    },
}

impl Services {
    pub fn new(config: Arc<ServiceConfiguration>, app: App) -> Self {
        let name = config.name.to_string();
        let trigger = Triggers::new(config.trigger.clone());
        match &*config.service {
            ServiceTypeConfiguration::Command { command, args } => Services::Command {
                name,
                trigger,
                service: CommandService {
                    app,
                    command: command.to_string(),
                    args: args.clone(),
                },
            },
            ServiceTypeConfiguration::Bluetooth { .. } => Services::Bluetooth {
                name,
                trigger,
                service: BluetoothService { app },
            },
        }
    }

    pub async fn run(&self) -> Result<()> {
        match self {
            Services::Command { service, name, .. } => service.run(name.to_string()).await,
            Services::Bluetooth { service, name, .. } => service.run(name.to_string()).await,
        }
    }

    pub fn start(&self) -> Result<()>{
        let svc = self.clone();
        match self {
            Services::Command { trigger, .. } => trigger.init(svc),
            Services::Bluetooth { trigger, .. } => trigger.init(svc),
        }
    }

    pub fn device_type(&self) -> &'static str {
        match self {
            Services::Command { service, .. } => service.device_type(),
            Services::Bluetooth { service, .. } => service.device_type(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Services::Command { name, .. } => name,
            Services::Bluetooth { name, .. } => name,
        }
    }
}
