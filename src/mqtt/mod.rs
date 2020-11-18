use crate::{prelude::*, service_interval, MQTTConfiguration, Result};
use cluster::ClusterState;
pub use devices::DeviceType;
pub use hass::{HassDeviceInformation, HassDiscoveryPayload};
use rumqttc::{self, AsyncClient, Event, EventLoop, Incoming, MqttOptions, QoS};
use std::sync::Arc;

mod cluster;
mod devices;
mod hass;

#[derive(Clone)]
pub struct MQTTService {
    location:  String,
    config:    Arc<MQTTConfiguration>,
    eventloop: SharedMutex<EventLoop>,
    client:    AsyncClient,
    cluster:   ClusterState,
}

impl std::fmt::Debug for MQTTService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MQTTService")
            .field("location", &self.location)
            .field("config", &self.config)
            .field("cluster", &self.cluster)
            .finish()
    }
}

impl MQTTService {
    pub fn new(location: String, config: Arc<MQTTConfiguration>) -> Self {
        let mut mqttoptions =
            MqttOptions::new(config.client_id.clone(), config.host.clone(), config.port);
        mqttoptions.set_keep_alive(5);

        let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
        MQTTService {
            location: location.clone(),
            config,
            client,
            eventloop: Arc::new(Mutex::new(eventloop)),
            cluster: ClusterState::new(location),
        }
    }

    async fn handle_message(&self, p: rumqttc::Publish) -> Result<()> {
        let payload = String::from_utf8(p.payload.to_vec()).unwrap_or_default();
        trace!("Payload received: '{}' => Topic: {}", payload, p.topic);
        match p.topic {
            t if t == format!("{}/cluster/leader", self.config.base_topic) => {
                self.cluster.set_leader(payload).await;
            }
            _ => warn!("Unknown topic"),
        }
        Ok(())
    }

    pub fn start(&self) -> Result<()> {
        let zelf = self.clone();
        spawn! {
            zelf.client
                .subscribe(
                    format!("{}/cluster/leader", zelf.config.base_topic),
                    QoS::AtLeastOnce,
                )
                .await?;
            let mut interval = tokio::time::interval(std::time::Duration::new(1, 0));
            loop {
                match zelf.eventloop.lock().await.poll().await {
                    Ok(Event::Incoming(Incoming::Publish(p))) => {
                        zelf.handle_message(p)
                            .await
                            .unwrap_or_else(|e| warn!("Error polling event: {:?}", e));
                    }
                    Err(e) => {
                        error!("Error received on MQTT poll: {:?}", e);
                        interval.tick().await;
                    }
                    Ok(_) => (),
                }
            }
        };

        let zelf = self.clone();
        service_interval!((10):{
            zelf.heartbeat().await?;
        });

        Ok(())
    }

    async fn heartbeat(&self) -> Result<()> {
        self.poll_leader().await?;
        Ok(())
    }

    async fn poll_leader(&self) -> Result<()> {
        if self.cluster.is_leader().await {
            trace!("Rebroadcasting leadership...");
            self.declare_leadership().await
        } else if self.cluster.leader_needed().await? {
            debug!("Cluster needs a leader, attempting to assume leadership...");
            self.declare_leadership().await
        } else {
            trace!(
                "Current cluster leader is '{}'",
                self.cluster
                    .get_leader()
                    .await?
                    .unwrap_or_else(|| "<none>".into())
            );
            Ok(())
        }
    }

    async fn declare_leadership(&self) -> Result<()> {
        self.client
            .publish(
                format!("{}/cluster/leader", self.config.base_topic),
                QoS::AtLeastOnce,
                true,
                self.location.to_string(),
            )
            .await?;
        Ok(())
    }

    // {"payload_available":"online","payload_not_available":"offline","availability_topic":"xx","icon":"mdi:account-group"}

    pub async fn add_device(
        &self,
        device_type: DeviceType,
        name: String,
        mut discovery: HassDiscoveryPayload,
    ) -> Result<()> {
        let t = format!(
            "{}/{}/{}/{}/config",
            self.config.discovery_topic,
            device_type,
            crate_name!(),
            name
        );

        let mut device_info = HassDeviceInformation::default();
        device_info.name = Some(name.to_string());
        device_info.model = Some(crate_name!().into());
        device_info.manufacturer = Some(crate_authors!().into());
        device_info.sw_version = Some(crate_version!().into());
        device_info.identifiers = Some(name.to_string());

        discovery.name = Some(name.to_string());
        discovery.device = Some(device_info);
        discovery.unique_id = Some(format!("{} {}", self.location, name));
        discovery.base_topic = Some(format!(
            "{}/nodes/{}/",
            self.config.base_topic, self.location
        ));

        discovery.state_topic = Some(format!("~/{}", name));
        discovery.json_attributes_topic = Some(format!("~/{}", name));
        self.client
            .publish(
                t,
                QoS::AtLeastOnce,
                true,
                serde_json::to_string(&discovery)?,
            )
            .await?;
        Ok(())
    }

    pub async fn send(&self, topic: String, payload: Document) -> Result<()> {
        let t = format!(
            "{}/nodes/{}/{}",
            self.config.base_topic, self.location, topic
        );
        trace!("MQTT send on topic {}", t);
        self.client
            .publish(t, QoS::AtLeastOnce, false, serde_json::to_string(&payload)?)
            .await?;
        Ok(())
    }
}
