use super::*;
use bluez::{
    client::*,
    interface::{controller::*, event::Event},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use tokio::time::{interval, Duration};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Reading {
    rssi:        i8,
    #[serde(with = "time_format")]
    timestamp:   DateTime<Utc>,
    mac_address: String,
}

#[derive(Clone, Debug)]
pub struct BluetoothPlugin {
    registry: DeviceRegistry,
    mqtt:     MQTTService,
    readings: SharedRwLock<HashMap<String, Reading>>,
}

impl BluetoothPlugin {
    pub fn new(registry: DeviceRegistry, mqtt: MQTTService) -> Self {
        Self {
            readings: Default::default(),
            registry,
            mqtt,
        }
    }
}

#[async_trait]
impl Plugin for BluetoothPlugin {
    async fn leader_heartbeat(&self, name: String, data: ClusterNodes) -> Result<()> {
        trace!("BluetoothService Leader Heartbeat");
        let mut node_data: HashMap<String, (String, i8, Document)> = Default::default();
        let ents = data.get_dev_id_prefix(&name).await;
        for (node, dev_id, stat) in ents.into_iter() {
            let latest_stat = stat.stat.get_latest().await.unwrap_or_default();
            let current = node_data.get(&dev_id);
            let rssi = i8::from_str(&latest_stat).unwrap_or(i8::MIN);
            match current {
                Some((_, old_rssi, _)) => {
                    if rssi > *old_rssi {
                        node_data.insert(dev_id, (node, rssi, stat.attr.clone()));
                    }
                }
                None => {
                    node_data.insert(dev_id, (node, rssi, stat.attr.clone()));
                }
            }
        }
        for (dev_id, (mut loc, rssi, attr)) in node_data.into_iter() {
            if rssi == i8::MIN {
                loc = "none".into();
            }
            self.registry
                .register(self.registry.new_device(
                    format!("{} Location", dev_id),
                    DeviceType::Sensor(SensorDeviceClass::None),
                    true,
                ))
                .await?;
            let d = self
                .registry
                .get_name(&format!("{} Location", dev_id))
                .await;
            let dev = DeviceUpdate {
                attr,
                device: d,
                value: loc.into(),
            };
            self.mqtt.update_device(&dev).await?;
        }
        Ok(())
    }

    async fn heartbeat(&self, name: String) -> Result<()> {
        trace!("BluetoothPlugin Heartbeat");
        let n = Utc::now();
        let readings = self.readings.read().await;
        for (mac, reading) in readings.iter() {
            self.registry
                .register(self.registry.new_device(
                    format!("{} {}", name, mac),
                    DeviceType::Sensor(SensorDeviceClass::SignalStrength),
                    false,
                ))
                .await?;
            if n.signed_duration_since(reading.timestamp).num_seconds() > 60 {
                let mut r = reading.clone();
                r.rssi = i8::MIN;
                let d = self
                    .registry
                    .get_name(&format!("{} {}", name, r.mac_address))
                    .await;
                let dev = DeviceUpdate {
                    device: d,
                    value:  r.rssi.into(),
                    attr:   Document::new(&r)?,
                };
                self.mqtt.update_device(&dev).await?;
            }
        }
        Ok(())
    }

    async fn run(&self, name: String) -> Result<()> {
        debug!("Initialize bluetooth plugin");
        let mut client = BlueZClient::new()?;

        let controllers = client.get_controller_list().await?;
        let mut controller = None;

        // find the first controller we can power on
        for c in controllers.into_iter() {
            let info = client.get_controller_info(c).await?;

            if info.supported_settings.contains(ControllerSetting::Powered) {
                debug!("Found bluetooth controller {:?}", info);
                controller = Some((c, info));
                break;
            }
        }

        if let Some((controller, info)) = controller {
            if !info.current_settings.contains(ControllerSetting::Powered) {
                info!("powering on bluetooth controller {}", controller);
                client.set_powered(controller, true).await?;
            }

            debug!("Starting bluetooth discovery...");
            client
                .start_discovery(
                    controller,
                    AddressTypeFlag::BREDR | AddressTypeFlag::LEPublic | AddressTypeFlag::LERandom,
                )
                .await?;

            let mut tick = interval(Duration::from_millis(50));
            loop {
                let response = client.process().await?;

                match response.event {
                        Event::DeviceFound {
                            address,
                            address_type,
                            rssi,
                            ..
                            // flags,
                        } => {
                            trace!("Bluetooth device found: {}", address);
                            if address_type == AddressType::BREDR {
                                debug!("Updating RSSI for {} to {}", address, rssi);
                                let mut r = self.readings.write().await;
                                let reading = Reading{rssi, timestamp: Utc::now(), mac_address: address.to_string()};
                                let d = self.registry.get_name(&format!("{} {}", name, address.to_string())).await;
                                let dev = DeviceUpdate {
                                    device: d,
                                    value: rssi.into(),
                                    attr:  Document::new(&reading)?,
                                };
                                self.mqtt.update_device(&dev).await?;
                                r.insert(address.to_string(), reading);
                            }
                        }
                        Event::Discovering {
                            discovering,
                            // address_type,
                            ..
                        } => {
                            // if discovery ended, turn it back on
                            trace!("Bluetooth discovery phase ended, restarting...");
                            if !discovering {
                                client
                                    .start_discovery(
                                        controller,
                                        AddressTypeFlag::BREDR
                                            | AddressTypeFlag::LEPublic
                                            | AddressTypeFlag::LERandom,
                                    )
                                    .await?;
                            }
                        }
                        e => error!("{:?}", e),
                    }

                tick.tick().await;
            }
        }
        Ok(())
    }
}
