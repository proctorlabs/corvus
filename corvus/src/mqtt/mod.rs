use crate::{prelude::*, util::StaticService, MQTTConfiguration, Result};
use async_trait::async_trait;
use chrono::prelude::*;
use cluster::ClusterState;
use rumqttc::{self, AsyncClient, Event, EventLoop, Incoming, LastWill, MqttOptions, QoS};
use std::{sync::Arc, time::Duration};

mod cluster;

#[derive(Clone, Deref)]
pub struct MQTTService(Arc<MQTTServiceData>);

pub struct MQTTServiceData {
    location:           String,
    availability_topic: String,
    nodes_topic:        String,
    leader_topic:       String,
    discovery_topic:    String,
    client:             SharedMutex<Option<AsyncClient>>,
    cluster:            ClusterState,
    cluster_data:       ClusterNodes,
    mqtt_options:       MqttOptions,
}

impl std::fmt::Debug for MQTTService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MQTTService")
            .field("location", &self.location)
            .field("cluster", &self.cluster)
            .finish()
    }
}

#[async_trait]
impl StaticService for MQTTService {
    const NAME: &'static str = "MQTT";
    const START_IMMEDIATELY: bool = true;
    const ADD_JITTER: bool = false;
    const DURATION: Duration = Duration::from_secs(10);

    async fn exec_service(zelf: Self) -> Result<()> {
        let mqtt = zelf.clone();
        let mut eventloop = mqtt.connect().await?;
        mqtt.publish(&mqtt.availability_topic, "online", true, QoS::AtLeastOnce)
            .await?;
        mqtt.subscribe(&format!("{}#", mqtt.nodes_topic), QoS::AtLeastOnce)
            .await?;
        mqtt.subscribe(&mqtt.leader_topic, QoS::AtLeastOnce).await?;

        // Main loop
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    mqtt.handle_message(p)
                        .await
                        .unwrap_or_else(|e| warn!("Error polling event: {:?}", e));
                }
                Err(e) => {
                    error!("Error received on MQTT poll: {:?}", e);
                    return mqtt.disconnect().await;
                }
                Ok(_) => (),
            }
        }
    }
}

impl MQTTService {
    pub async fn new(
        location: String,
        config: Arc<MQTTConfiguration>,
        cluster_data: ClusterNodes,
    ) -> Result<Self> {
        let cluster_topic = format!("{}/cluster/", config.base_topic);
        let leader_topic = format!("{}leader", cluster_topic);
        let nodes_topic = format!("{}/nodes/", config.base_topic);
        let availability_topic = format!("{}{}/avty", nodes_topic, clean_name(&location));
        let discovery_topic = config.discovery_topic.to_string();
        let mut mqtt_options =
            MqttOptions::new(config.client_id.clone(), config.host.clone(), config.port);
        mqtt_options.set_keep_alive(5);
        mqtt_options.set_last_will(LastWill::new(
            &availability_topic,
            QoS::AtLeastOnce,
            "offline",
        ));

        Ok(MQTTService(Arc::new(MQTTServiceData {
            cluster: ClusterState::new(location.clone()),
            client: Arc::new(Mutex::new(None)),
            cluster_data,
            location,
            discovery_topic,
            nodes_topic,
            availability_topic,
            leader_topic,
            mqtt_options,
        })))
    }

    pub async fn connect(&self) -> Result<EventLoop> {
        info!("Connecting to MQTT broker");
        let (client, eventloop) = AsyncClient::new(self.mqtt_options.clone(), 10);
        let mut cli_lock = self.client.lock().await;
        *cli_lock = Some(client);
        info!("MQTT connected");
        Ok(eventloop)
    }

    pub async fn disconnect(&self) -> Result<()> {
        warn!("MQTT Disconnected");
        let mut cli_lock = self.client.lock().await;
        *cli_lock = None;
        Ok(())
    }

    async fn handle_message(&self, p: rumqttc::Publish) -> Result<()> {
        let payload = String::from_utf8(p.payload.to_vec()).unwrap_or_default();
        trace!("Payload received: '{}' => Topic: {}", payload, p.topic);
        match p.topic {
            t if t == self.leader_topic => {
                self.cluster.set_leader(payload).await;
            }
            t if t.starts_with(&self.nodes_topic) => {
                self.handle_node_update(&t, payload).await?;
            }
            t => debug!("Unknown topic '{}'", t),
        }
        Ok(())
    }

    async fn handle_node_update(&self, topic: &str, payload: String) -> Result<()> {
        let suffix = topic.trim_start_matches(&self.nodes_topic);
        let suffix_parts: Vec<&str> = suffix.split('/').collect();
        if suffix_parts.len() == 3 {
            let (node, uniq_id, typ) = (suffix_parts[0], suffix_parts[1], suffix_parts[2]);
            let node_prefix = format!("{}_", clean_name(node));
            let dev_id = uniq_id.trim_start_matches(&node_prefix);
            match typ {
                "stat" => self.cluster_data.update_stat(node, dev_id, payload).await,
                "attr" => {
                    self.cluster_data
                        .update_attr(node, dev_id, serde_json::from_str(&payload)?)
                        .await
                }
                _ => (),
            }
        }
        Ok(())
    }

    pub async fn heartbeat(&self) -> Result<()> {
        self.poll_leader().await
    }

    pub async fn is_leader(&self) -> bool {
        self.cluster.is_leader().await
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

    pub async fn publish(&self, topic: &str, message: &str, retain: bool, qos: QoS) -> Result<()> {
        match self.client.lock().await.as_ref() {
            Some(c) => {
                c.publish(topic, qos, retain, message).await?;
                Ok(())
            }
            None => Err(anyhow!("Not connected!")),
        }
    }

    pub async fn subscribe(&self, topic: &str, qos: QoS) -> Result<()> {
        match self.client.lock().await.as_ref() {
            Some(c) => {
                c.subscribe(topic, qos).await?;
                Ok(())
            }
            None => Err(anyhow!("Not connected!")),
        }
    }

    async fn declare_leadership(&self) -> Result<()> {
        self.publish(
            &self.leader_topic,
            &self.cluster.get_sid().await?,
            true,
            QoS::AtLeastOnce,
        )
        .await?;
        Ok(())
    }

    pub async fn add_device(&self, device: &Device) -> Result<()> {
        let payload = device.to_discovery();
        let t = format!(
            "{}/{}/{}/{}/config",
            self.discovery_topic,
            device.device_type(),
            crate_name!(),
            device.uniq_id(),
        );
        self.publish(
            &t,
            &serde_json::to_string(&payload)?,
            true,
            QoS::AtLeastOnce,
        )
        .await?;
        Ok(())
    }

    pub async fn update_device(&self, d: &DeviceUpdate) -> Result<()> {
        if let Some(device) = &d.device {
            self.publish(
                &device.stat_topic(),
                &d.value.to_string(),
                false,
                QoS::AtLeastOnce,
            )
            .await?;

            let mut attr = d.attr.clone();
            attr["set_by_location"] = self.location.clone().into();
            attr["update_timestamp"] = Local::now().to_rfc3339().into();
            self.publish(
                &device.attr_topic(),
                &serde_json::to_string(&attr)?,
                false,
                QoS::AtLeastOnce,
            )
            .await?;
        }
        Ok(())
    }
}
