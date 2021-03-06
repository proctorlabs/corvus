pub use crate::prelude::*;
use crate::{mqtt::MQTTService, plugins::PluginManager};
use std::time::Duration;

mod args;

#[derive(Clone, Deref, Debug)]
pub struct App(Arc<AppServices>);

#[derive(Debug)]
pub struct AppServices {
    pub config:          Arc<Configuration>,
    pub mqtt:            MQTTService,
    pub plugin_manager:  PluginManager,
    pub cluster_data:    ClusterNodes,
    pub device_registry: DeviceRegistry,
}

impl App {
    pub async fn new() -> Result<Self> {
        let opts = args::parse()?;
        simplelog::TermLogger::init(
            match opts.verbosity {
                0 => simplelog::LevelFilter::Info,
                1 => simplelog::LevelFilter::Debug,
                _ => simplelog::LevelFilter::Trace,
            },
            simplelog::ConfigBuilder::new()
                .add_filter_allow_str("corvus")
                .set_location_level(simplelog::LevelFilter::Debug)
                .set_target_level(simplelog::LevelFilter::Error)
                .set_time_format_str("%D %T")
                .set_time_to_local(true)
                .build(),
            simplelog::TerminalMode::Mixed,
        )?;
        if opts.generate {
            info!(
                "Generating new configuration file at {}",
                opts.config.to_string_lossy()
            );
            Configuration::generate_default(opts.config)?;
            std::process::exit(0);
        } else {
            info!("Starting Corvus");
            let config = Configuration::load(opts.config)?;
            let cluster_data = ClusterNodes::default();
            let plugin_manager: PluginManager = Default::default();
            let mqtt_service = MQTTService::new(
                config.node.location.clone(),
                config.mqtt.clone(),
                plugin_manager.clone(),
            )
            .await?;
            Ok(App(Arc::new(AppServices {
                device_registry: DeviceRegistry::new(mqtt_service.clone(), config.clone()),
                plugin_manager,
                mqtt: mqtt_service,
                cluster_data,
                config,
            })))
        }
    }

    async fn init_heartbeats(&self) -> Result<()> {
        let zelf = self.clone();
        start_service(
            Duration::from_secs(10),
            "MQTT Heartbeat".into(),
            false,
            true,
            move || {
                let zelf = zelf.clone();
                async move { zelf.mqtt.heartbeat().await }
            },
        )?;

        for svc in self.plugin_manager.lock().await.values().cloned() {
            let svc = svc.clone();
            let zelf = self.clone();
            start_service(
                Duration::from_secs(10),
                format!("{} Plugin Heartbeat", svc.name()),
                false,
                false,
                move || {
                    let svc = svc.clone();
                    let zelf = zelf.clone();
                    async move {
                        svc.heartbeat().await?;
                        if zelf.mqtt.is_leader().await {
                            svc.leader_heartbeat(zelf.cluster_data.clone()).await?;
                        }
                        Ok(())
                    }
                },
            )?;
        }
        Ok(())
    }

    async fn init_plugins(&self) -> Result<()> {
        self.plugin_manager.init_plugins(&self.config, self).await
    }

    async fn init_mqtt(&self) -> Result<()> {
        self.mqtt.start_service()
    }

    async fn init_device_registry(&self) -> Result<()> {
        self.device_registry.start_service()
    }

    pub async fn start(&self) -> Result<()> {
        self.init_mqtt().await?;
        self.init_plugins().await?;
        self.init_heartbeats().await?;
        self.init_device_registry().await?;
        tokio::signal::ctrl_c().await?;
        warn!("Signal received, shutting down");
        Ok(())
    }
}
