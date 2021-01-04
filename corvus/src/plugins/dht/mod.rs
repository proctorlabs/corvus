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
    mqtt:               MQTTService,
    registry:           DeviceRegistry,
    dht:                DHT,
    temperature_device: String,
    humidity_device:    String,
}

impl DHTPlugin {
    pub fn new(
        name: String,
        mqtt: MQTTService,
        registry: DeviceRegistry,
        device: String,
        channel: u32,
    ) -> Self {
        DHTPlugin {
            dht: DHT::new(&device, channel).unwrap(),
            temperature_device: format!("{} Temperature", name),
            humidity_device: format!("{} Humidity", name),
            mqtt,
            registry,
        }
    }
}

#[async_trait]
impl Plugin for DHTPlugin {
    async fn leader_heartbeat(&self, _: String, _: ClusterNodes) -> Result<()> {
        Ok(())
    }

    async fn heartbeat(&self, name: String) -> Result<()> {
        self.registry
            .register(
                self.registry
                    .new_device(
                        self.temperature_device.to_string(),
                        DeviceType::Sensor(SensorDeviceClass::Temperature),
                        name.to_string(),
                    )
                    .with_unit_of_measurement("°C".into())
                    .build(),
            )
            .await?;
        self.registry
            .register(
                self.registry
                    .new_device(
                        self.humidity_device.to_string(),
                        DeviceType::Sensor(SensorDeviceClass::Humidity),
                        name,
                    )
                    .with_unit_of_measurement("%".into())
                    .build(),
            )
            .await?;
        Ok(())
    }

    async fn run(&self, _: String) -> Result<()> {
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
                let d = self.registry.get_by_name(&self.humidity_device).await;
                let update = DeviceUpdate {
                    device: d,
                    value:  r.humidity.into(),
                    attr:   Default::default(),
                };
                self.mqtt.update_device(&update).await?;
                let d = self.registry.get_by_name(&self.temperature_device).await;
                let update = DeviceUpdate {
                    device: d,
                    value:  r.temperature.into(),
                    attr:   Default::default(),
                };
                self.mqtt.update_device(&update).await?;
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
