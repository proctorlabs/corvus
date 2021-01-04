mod devices;
mod hass;

use crate::prelude::*;
pub use devices::*;
pub use hass::*;
use std::{collections::HashMap, time::Duration};

#[derive(Clone, Debug, Deref)]
pub struct DeviceRegistry(DeviceRegistryInner);

#[derive(Clone, Debug)]
pub struct DeviceRegistryInner {
    devices_names: Arc<RwLock<HashMap<String, Device>>>,
    devices_ids:   Arc<RwLock<HashMap<String, Device>>>,
    mqtt:          MQTTService,
    location:      String,
    base_topic:    String,
}

#[async_trait]
impl StaticService for DeviceRegistry {
    const NAME: &'static str = "Device Registry";
    const START_IMMEDIATELY: bool = false;
    const ADD_JITTER: bool = false;
    const DURATION: Duration = Duration::from_secs(120);

    async fn exec_service(zelf: Self) -> Result<()> {
        zelf.publish_all().await
    }
}

impl DeviceRegistry {
    pub fn new(mqtt: MQTTService, config: Arc<Configuration>) -> Self {
        Self(DeviceRegistryInner {
            devices_ids: Default::default(),
            devices_names: Default::default(),
            location: config.node.location.to_string(),
            base_topic: config.mqtt.base_topic.to_string(),
            mqtt,
        })
    }

    pub fn new_device(&self, display_name: String, typ: DeviceType, plugin: String) -> DeviceData {
        DeviceData::new(
            display_name,
            typ,
            self.location.to_string(),
            self.base_topic.to_string(),
            plugin,
        )
    }

    pub async fn get_by_name(&self, key: &str) -> Option<Device> {
        let reg = self.devices_names.read().await;
        let d = reg.get(key).cloned();
        d
    }

    pub async fn get_by_id(&self, key: &str) -> Option<Device> {
        let reg = self.devices_names.read().await;
        let d = reg.get(key).cloned();
        d
    }

    pub async fn register(&self, device: Device) -> Result<Device> {
        let mut reg = self.devices_names.write().await;
        reg.insert(device.display_name().to_string(), device.clone());
        drop(reg);
        let mut reg = self.devices_ids.write().await;
        let existing = reg.insert(device.id().to_string(), device.clone());
        drop(reg);

        if existing.is_none() {
            self.publish_device(&device).await?;
        }
        Ok(device)
    }

    pub async fn list_devices(&self) -> Result<Vec<Device>> {
        let reg = self.devices_names.read().await;
        Ok(reg.values().cloned().collect())
    }

    pub async fn publish_all(&self) -> Result<()> {
        for d in self.list_devices().await? {
            self.publish_device(&d).await?;
        }
        Ok(())
    }

    async fn publish_device(&self, device: &Device) -> Result<()> {
        self.mqtt.add_device(device).await?;
        Ok(())
    }
}
