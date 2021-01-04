use super::*;
use bluez::{
    client::*,
    interface::{controller::*, event::Event},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use tokio::time::{sleep, Duration};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Reading {
    rssi:        i8,
    mac_address: String,
    #[serde(with = "time_format")]
    timestamp:   DateTime<Utc>,
}

type NodeReadings = HashMap<String, RollingVec<Reading>>;

#[derive(Clone, Debug)]
pub struct BluetoothPlugin {
    registry: DeviceRegistry,
    mqtt:     MQTTService,
    location: String,
    readings: SharedRwLock<HashMap<String, Reading>>,
    nodes:    SharedRwLock<HashMap<String, NodeReadings>>,
    name:     String,
}

#[derive(Debug)]
enum BTDeviceType {
    Rssi(String),
    Location(String),
}

impl BluetoothPlugin {
    pub fn new(
        name: String,
        location: String,
        registry: DeviceRegistry,
        mqtt: MQTTService,
    ) -> Self {
        Self {
            readings: Default::default(),
            nodes: Default::default(),
            name,
            location,
            registry,
            mqtt,
        }
    }

    async fn get_device(&self, typ: BTDeviceType) -> Result<Device> {
        let name = match typ {
            BTDeviceType::Rssi(ref mac) => format!("{} {} {}", self.location, self.name, mac),
            BTDeviceType::Location(ref mac) => format!("{} {} Location", self.name, mac),
        };
        let device = self.registry.get_by_name(&name).await;
        if let Some(device) = device {
            Ok(device)
        } else {
            let device = match typ {
                BTDeviceType::Rssi(_) => self
                    .registry
                    .new_device(
                        name,
                        DeviceType::Sensor(SensorDeviceClass::SignalStrength),
                        self.name.to_string(),
                    )
                    .with_unit_of_measurement("dBm".into())
                    .build(),
                BTDeviceType::Location(_) => self
                    .registry
                    .new_device(
                        name,
                        DeviceType::Sensor(SensorDeviceClass::None),
                        self.name.to_string(),
                    )
                    .into_cluster_device()
                    .build(),
            };
            self.registry.register(device).await
        }
    }
}

#[async_trait]
impl Plugin for BluetoothPlugin {
    async fn leader_heartbeat(&self, _: String, _: ClusterNodes) -> Result<()> {
        trace!("BluetoothService Leader Heartbeat");

        let mut mac_locations: HashMap<String, String> = Default::default();
        for (mac, map) in self.nodes.read().await.iter() {
            let mut loc_readings = ("Unknown", i8::MIN);
            for (location, readings) in map.iter() {
                let rssi = readings
                    .get_all()
                    .await
                    .iter()
                    .rev()
                    .take(3)
                    .fold((0, 0), |acc, x| (acc.0 + (x.rssi as i64), acc.1 + 1));
                if rssi.1 > 0 {
                    // debug!("{:?}", rssi);
                    let rssi = (rssi.0 / rssi.1) as i8;
                    // debug!("{}", rssi);
                    if rssi > loc_readings.1 {
                        loc_readings = (location, rssi);
                    }
                }
            }
            mac_locations.insert(mac.into(), loc_readings.0.into());
        }
        trace!("{:?}", mac_locations);

        for (mac, location) in mac_locations.into_iter() {
            let d = self.get_device(BTDeviceType::Location(mac)).await?;
            let mut attr = Document::default();
            attr["device_location"] = location.to_string().into();
            let dev = DeviceUpdate {
                device: Some(d),
                value: location.into(),
                attr,
            };
            self.mqtt.update_device(&dev).await?;
        }
        Ok(())
    }

    async fn heartbeat(&self, _: String) -> Result<()> {
        trace!("BluetoothPlugin Heartbeat");
        let n = Utc::now();
        let readings = self.readings.read().await;
        for (mac, reading) in readings.iter() {
            let d = self.get_device(BTDeviceType::Rssi(mac.to_string())).await?;
            if n.signed_duration_since(reading.timestamp).num_seconds() > 60 {
                let mut r = reading.clone();
                r.rssi = i8::MIN;
                let dev = DeviceUpdate {
                    device: Some(d),
                    value:  r.rssi.into(),
                    attr:   Document::new(&r)?,
                };
                self.mqtt.update_device(&dev).await?;
            }
        }
        Ok(())
    }

    async fn process_update(&self, data: Document) -> Result<()> {
        if data["rssi_type"] == Document::String("single_reading".into()) {
            let location = data["corvus_location"].to_string();
            let rssi = data["rssi"].as_i64().unwrap_or_default();
            let mac_address = data["mac_address"].to_string();
            let reading = Reading {
                timestamp:   Utc::now(),
                rssi:        rssi as i8,
                mac_address: mac_address.to_string(),
            };
            let mut nodes = self.nodes.write().await;
            if !nodes.contains_key(&mac_address) {
                nodes.insert(mac_address.clone(), Default::default());
            }
            let readings = nodes.get_mut(&mac_address).unwrap();
            let rv = readings.get(&location);
            let rv = if let Some(rv) = rv {
                rv
            } else {
                let rv = RollingVec::new(Duration::from_secs(90));
                readings.insert(location.to_string(), rv);
                readings.get(&location).unwrap()
            };
            rv.add(reading).await;
        }
        Ok(())
    }

    async fn run(&self, _: String) -> Result<()> {
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
                                let reading = Reading{rssi, timestamp: Utc::now(), mac_address: address.to_string()};
                                let d = self.get_device(BTDeviceType::Rssi(address.to_string())).await?;
                                let mut attr = Document::new(&reading)?;
                                attr["rssi_type"] = "single_reading".into();
                                let dev = DeviceUpdate {
                                    device: Some(d),
                                    value: rssi.into(),
                                    attr,
                                };
                                self.mqtt.update_device(&dev).await?;
                                let mut r = self.readings.write().await;
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
                        e => {
                            error!("{:?}", e);
                        }
                    }

                sleep(Duration::from_millis(50)).await;
            }
        }
        Ok(())
    }
}
