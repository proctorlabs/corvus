use super::*;
use bluez::{
    client::*,
    interface::{controller::*, event::Event},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::{interval, Duration};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Reading {
    rssi:        i8,
    #[serde(with = "time_format")]
    timestamp:   DateTime<Utc>,
    mac_address: String,
}

#[derive(Clone, Debug)]
pub struct BluetoothService {
    pub app:  App,
    readings: SharedRwLock<HashMap<String, Reading>>,
}

impl BluetoothService {
    pub fn new(app: App) -> Self {
        BluetoothService {
            readings: Default::default(),
            app,
        }
    }
}

#[async_trait]
impl Service for BluetoothService {
    async fn heartbeat(&self, name: String) -> Result<()> {
        trace!("BluetoothService Heartbeat");
        let n = Utc::now();
        let readings = self.readings.read().await;
        for (mac, reading) in readings.iter() {
            self.app
                .mqtt
                .add_device(DeviceInfo {
                    name: format!("{}_{}", name, mac),
                    typ:  DeviceType::Sensor,
                })
                .await?;
            if n.signed_duration_since(reading.timestamp).num_seconds() > 60 {
                let mut r = reading.clone();
                r.rssi = i8::MIN;
                let dev = DeviceUpdate {
                    name:  format!("{}_{}", name, r.mac_address),
                    value: r.rssi.into(),
                    attr:  Some(Document::new(&r)?),
                };
                self.app.mqtt.update_device(&dev).await?;
            }
        }
        Ok(())
    }

    async fn run(&self, name: String) -> Result<()> {
        debug!("Initialize bluetooth services");
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
                                let dev = DeviceUpdate {
                                    name:  format!("{}_{}", name, address.to_string()),
                                    value: rssi.into(),
                                    attr:  Some(Document::new(&reading)?),
                                };
                                self.app.mqtt.update_device(&dev).await?;
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
