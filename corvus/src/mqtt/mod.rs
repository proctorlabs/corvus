use crate::{prelude::*, MQTTConfiguration, Result};
use cluster::ClusterState;
use rumqttc::{self, AsyncClient, Event, EventLoop, Incoming, LastWill, MqttOptions, QoS};
use std::{sync::Arc, time::Duration};

mod cluster;

fn normalize_name(name: &str) -> String {
    name.replace(":", "").replace("-", "_")
}

#[derive(Clone, Deref)]
pub struct MQTTService(Arc<MQTTServiceData>);

pub struct MQTTServiceData {
    location:           String,
    availability_topic: String,
    nodes_topic:        String,
    leader_topic:       String,
    location_topic:     String,
    discovery_topic:    String,
    cluster_topic:      String,
    base_topic:         String,
    eventloop:          SharedMutex<EventLoop>,
    client:             AsyncClient,
    cluster:            ClusterState,
    cluster_data:       ClusterNodes,
}

impl std::fmt::Debug for MQTTService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MQTTService")
            .field("location", &self.location)
            .field("cluster", &self.cluster)
            .finish()
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
        let availability_topic = format!("{}{}/avty", nodes_topic, location);
        let location_topic = format!("{}{}/", nodes_topic, location);
        let discovery_topic = config.discovery_topic.to_string();
        let base_topic = config.base_topic.to_string();
        let mut mqttoptions =
            MqttOptions::new(config.client_id.clone(), config.host.clone(), config.port);
        mqttoptions.set_keep_alive(5);
        mqttoptions.set_last_will(LastWill::new(
            &availability_topic,
            QoS::AtLeastOnce,
            "offline",
        ));
        let (client, eventloop) = AsyncClient::new(mqttoptions, 10);

        Ok(MQTTService(Arc::new(MQTTServiceData {
            eventloop: Arc::new(Mutex::new(eventloop)),
            cluster: ClusterState::new(location.clone()),
            base_topic,
            cluster_data,
            location,
            client,
            discovery_topic,
            nodes_topic,
            availability_topic,
            leader_topic,
            location_topic,
            cluster_topic,
        })))
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
            let node_prefix = format!("{}_", normalize_name(node));
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

    pub fn start(&self) -> Result<()> {
        let mqtt = self.clone();
        start_service(
            Duration::from_secs(2),
            "MQTT Service".into(),
            true,
            move || {
                let mqtt = mqtt.clone();
                async move {
                    mqtt.client
                        .publish(&mqtt.availability_topic, QoS::AtLeastOnce, true, "online")
                        .await?;
                    mqtt.client
                        .subscribe(format!("{}#", mqtt.nodes_topic), QoS::AtLeastOnce)
                        .await?;
                    mqtt.client
                        .subscribe(&mqtt.leader_topic, QoS::AtLeastOnce)
                        .await?;

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
                }
            },
        )?;
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

    async fn declare_leadership(&self) -> Result<()> {
        self.client
            .publish(
                &self.leader_topic,
                QoS::AtLeastOnce,
                true,
                self.cluster.get_sid().await?,
            )
            .await?;
        Ok(())
    }

    fn get_id(&self, name: &str) -> Result<String> {
        Ok(normalize_name(&format!("{}_{}", self.location, name)))
    }

    pub async fn add_device(&self, device: &Device) -> Result<()> {
        let payload = device.to_discovery(self.location.to_string(), self.base_topic.to_string());
        let t = format!(
            "{}/{}/{}/{}/config",
            self.discovery_topic,
            device.device_type(),
            crate_name!(),
            payload.unique_id.clone().unwrap_or_default()
        );
        self.client
            .publish(t, QoS::AtLeastOnce, true, serde_json::to_string(&payload)?)
            .await?;
        Ok(())
    }

    pub async fn update_device(&self, d: &DeviceUpdate) -> Result<()> {
        let loc_t = if d.is_cluster_device {
            format!("{}{}/", self.cluster_topic, normalize_name(&d.name))
        } else {
            format!("{}{}/", self.location_topic, self.get_id(&d.name)?)
        };
        let stat_t = format!("{}stat", loc_t);
        self.client
            .publish(stat_t, QoS::AtLeastOnce, false, d.value.to_string())
            .await?;

        self.client
            .publish(
                format!("{}attr", loc_t),
                QoS::AtLeastOnce,
                false,
                serde_json::to_string(&d.attr)?,
            )
            .await?;
        Ok(())
    }
}
