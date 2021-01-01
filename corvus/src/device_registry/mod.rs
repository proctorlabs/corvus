mod devices;
mod hass;

use crate::prelude::*;
pub use devices::*;
pub use hass::*;
use std::collections::HashMap;

#[derive(Clone, Debug, Deref)]
pub struct DeviceRegistry(DeviceRegistryInner);

#[derive(Clone, Debug)]
pub struct DeviceRegistryInner {
    devices: Arc<RwLock<HashMap<String, Arc<Device>>>>,
    mqtt:    MQTTService,
}

impl DeviceRegistry {
    pub fn new(mqtt: MQTTService) -> Self {
        Self(DeviceRegistryInner {
            devices: Default::default(),
            mqtt,
        })
    }

    pub async fn register(&self, device: Device) -> Result<()> {
        let mut reg = self.devices.write().await;
        let device = Arc::new(device);
        let existing = reg.insert(device.id().to_string(), device.clone());
        if existing.is_none() {
            self.publish_device(&device).await?;
        }
        Ok(())
    }

    pub async fn list_devices(&self) -> Result<Vec<Arc<Device>>> {
        let reg = self.devices.read().await;
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
