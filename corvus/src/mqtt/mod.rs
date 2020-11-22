use crate::{prelude::*, MQTTConfiguration, Result};
use cluster::ClusterState;
use rumqttc::{self, AsyncClient, Event, EventLoop, Incoming, LastWill, MqttOptions, QoS};
use std::sync::Arc;

mod cluster;

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
    pub async fn new(location: String, config: Arc<MQTTConfiguration>) -> Result<Self> {
        let mut mqttoptions =
            MqttOptions::new(config.client_id.clone(), config.host.clone(), config.port);
        mqttoptions.set_keep_alive(5);
        let avty_t = format!("{}/nodes/{}/avty", config.base_topic, location);
        mqttoptions.set_last_will(LastWill::new(&avty_t, QoS::AtLeastOnce, "offline"));
        let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
        client
            .publish(avty_t, QoS::AtLeastOnce, true, "online")
            .await?;

        Ok(MQTTService {
            location: location.clone(),
            config,
            client,
            eventloop: Arc::new(Mutex::new(eventloop)),
            cluster: ClusterState::new(location),
        })
    }

    async fn handle_message(&self, p: rumqttc::Publish) -> Result<()> {
        let payload = String::from_utf8(p.payload.to_vec()).unwrap_or_default();
        trace!("Payload received: '{}' => Topic: {}", payload, p.topic);
        match p.topic {
            t if t == format!("{}/cluster/leader", self.config.base_topic) => {
                self.cluster.set_leader(payload).await;
            }
            _ => trace!("Unknown topic"),
        }
        Ok(())
    }

    pub fn start(&self) -> Result<()> {
        let mqtt = self.clone();
        spawn! {
            // New connection setup
            mqtt.client
                .subscribe(
                    format!("{}/cluster/leader", mqtt.config.base_topic),
                    QoS::AtLeastOnce,
                )
                .await?;

            // mqtt.client.publish();

            // Main loop
            let mut interval = tokio::time::interval(std::time::Duration::new(1, 0));
            loop {
                match mqtt.eventloop.lock().await.poll().await {
                    Ok(Event::Incoming(Incoming::Publish(p))) => {
                        mqtt.handle_message(p)
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
        Ok(())
    }

    pub async fn heartbeat(&self) -> Result<()> {
        self.poll_leader().await
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
                self.cluster.get_sid().await?,
            )
            .await?;
        Ok(())
    }

    fn get_id(&self, name: &str) -> Result<String> {
        Ok(format!("{}_{}", self.location, name)
            .replace(":", "")
            .replace("-", "_"))
    }

    // {"payload_available":"online","payload_not_available":"offline","availability_topic":"xx","icon":"mdi:account-group"}

    pub async fn add_device(&self, device: DeviceInfo) -> Result<()> {
        let uniq_id = self.get_id(&device.name)?;
        let t = format!(
            "{}/{}/{}/{}/config",
            self.config.discovery_topic,
            device.typ,
            crate_name!(),
            uniq_id
        );

        let mut device_info = HassDeviceInformation::default();
        device_info.name = Some(self.location.to_string());
        device_info.model = Some(crate_name!().into());
        device_info.manufacturer = Some(crate_authors!().into());
        device_info.sw_version = Some(crate_version!().into());
        device_info.identifiers = Some(self.location.replace(":", "").replace("-", "_"));

        let mut discovery = HassDiscoveryPayload::default();
        discovery.name = Some(device.name.to_string());
        discovery.device = Some(device_info);
        discovery.unique_id = Some(uniq_id.to_string());
        discovery.base_topic = Some(format!(
            "{}/nodes/{}/{}/",
            self.config.base_topic, self.location, uniq_id
        ));

        discovery.state_topic = Some("~stat".to_string());
        discovery.json_attributes_topic = Some("~attr".to_string());
        discovery.availability_topic = Some(format!(
            "{}/nodes/{}/avty",
            self.config.base_topic, self.location
        ));
        discovery.payload_available = Some("online".into());
        discovery.payload_not_available = Some("offline".into());
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

    pub async fn update_device(&self, d: &DeviceUpdate) -> Result<()> {
        let uniq_id = self.get_id(&d.name)?;
        let stat_t = format!(
            "{}/nodes/{}/{}/stat",
            self.config.base_topic, self.location, uniq_id
        );
        self.client
            .publish(stat_t, QoS::AtLeastOnce, false, d.value.to_string())
            .await?;
        if let Some(attr) = &d.attr {
            let attr_t = format!(
                "{}/nodes/{}/{}/attr",
                self.config.base_topic, self.location, uniq_id
            );
            self.client
                .publish(
                    attr_t,
                    QoS::AtLeastOnce,
                    false,
                    serde_json::to_string(attr)?,
                )
                .await?;
        }
        Ok(())
    }

    // pub async fn send(&self, topic: String, payload: Document) -> Result<()> {
    //     let t = format!(
    //         "{}/nodes/{}/{}",
    //         self.config.base_topic, self.location, topic
    //     );
    //     trace!("MQTT send on topic {}", t);
    //     self.client
    //         .publish(t, QoS::AtLeastOnce, false, serde_json::to_string(&payload)?)
    //         .await?;
    //     Ok(())
    // }
}
