use super::*;
use dht22::*;
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use unstructured::Document;

mod dht22;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct DHTPayload {
    humidity:    f32,
    temperature: f32,
}

#[derive(Debug, Clone)]
pub struct DHTPlugin {
    app: App,
    dht: DHT,
}

impl DHTPlugin {
    pub fn new(app: App, device: String, channel: u32) -> Self {
        DHTPlugin {
            app,
            dht: DHT::new(&device, channel).unwrap(),
        }
    }
}

#[async_trait]
impl Plugin for DHTPlugin {
    async fn leader_heartbeat(&self, _: String, _: ClusterNodes) -> Result<()> {
        Ok(())
    }

    async fn heartbeat(&self, name: String) -> Result<()> {
        self.app
            .mqtt
            .add_device(DeviceInfo {
                name:              format!("{}_temperature", name),
                typ:               DeviceType::Sensor,
                is_cluster_device: false,
            })
            .await?;
        self.app
            .mqtt
            .add_device(DeviceInfo {
                name:              format!("{}_humidity", name),
                typ:               DeviceType::Sensor,
                is_cluster_device: false,
            })
            .await?;
        Ok(())
    }

    async fn run(&self, name: String) -> Result<()> {
        let mut zelf = self.clone();
        match Handle::current()
            .spawn_blocking(move || {
                let mut last_result = zelf.dht.get_reading();
                let mut i = 0;
                while last_result.is_err() && i < 10 {
                    last_result = zelf.dht.get_reading();
                    i += 1;
                }
                last_result
            })
            .await?
        {
            Ok(r) => {
                let update = DeviceUpdate {
                    name:              format!("{}_humidity", name),
                    value:             r.humidity.into(),
                    attr:              Some(Document::String("".into())),
                    is_cluster_device: false,
                };
                self.app.mqtt.update_device(&update).await?;
                let update = DeviceUpdate {
                    name:              format!("{}_temperature", name),
                    value:             r.temperature.into(),
                    attr:              Some(Document::String("".into())),
                    is_cluster_device: false,
                };
                self.app.mqtt.update_device(&update).await?;
            }
            Err(e) => {
                warn!("Read device failed due to {:?}", e);
            }
        };
        Ok(())
    }
}

impl From<DHTPayload> for Document {
    fn from(m: DHTPayload) -> Self {
        Document::new(m).unwrap_or_default()
    }
}
