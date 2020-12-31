mod devices;

use crate::prelude::*;
pub use devices::*;
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
        reg.insert(device.id().to_string(), Arc::new(device));
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
