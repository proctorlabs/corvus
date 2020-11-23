#[macro_use]
extern crate clap;

#[macro_use]
extern crate derive_more;

#[macro_use]
extern crate log;

#[macro_use]
pub(crate) mod macros;

mod args;
mod config;
mod data_structures;
mod mqtt;
mod prelude;
mod services;
mod triggers;

use mqtt::MQTTService;
pub use prelude::*;

#[tokio::main(core_threads = 6)]
async fn main() -> Result<()> {
    App::new().await?.start().await
}

#[derive(Clone, Deref, Debug)]
pub struct App(Arc<AppServices>);

#[derive(Debug)]
pub struct AppServices {
    pub config:       Arc<Configuration>,
    pub mqtt:         MQTTService,
    pub services:     Mutex<Vec<services::Services>>,
    pub cluster_data: ClusterNodes,
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
            let mqtt_service = MQTTService::new(
                config.node.location.clone(),
                config.mqtt.clone(),
                cluster_data.clone(),
            )
            .await?;
            Ok(App(Arc::new(AppServices {
                config,
                services: Mutex::new(vec![]),
                mqtt: mqtt_service,
                cluster_data,
            })))
        }
    }

    async fn heartbeat(&self) -> Result<()> {
        trace!("Running MQTT heartbeat");
        self.mqtt.heartbeat().await?;
        let is_leader = self.mqtt.is_leader().await;
        for svc in &*self.services.lock().await {
            trace!("Running service heartbeat: {:?}", svc.name());
            svc.heartbeat().await?;
            if is_leader {
                svc.leader_heartbeat(self.cluster_data.clone()).await?;
            }
        }
        if self.mqtt.is_leader().await {}
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        self.mqtt.start()?;
        let mut svcs = self.services.lock().await;
        for svc in self.config.services.iter() {
            let s = services::Services::new(svc.clone(), self.clone());
            s.start()?;
            svcs.insert(0, s);
        }
        drop(svcs);
        let zelf = self.clone();
        service_interval!((10):{
            zelf.heartbeat().await?;
        });
        tokio::signal::ctrl_c().await?;
        warn!("Signal received, shutting down");
        Ok(())
    }
}
