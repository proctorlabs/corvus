use super::*;
use bluez::client::*;
use bluez::interface::controller::*;
use bluez::interface::event::Event;
use tokio::time::{interval, Duration};

#[derive(Clone, Debug)]
pub struct BluetoothService {
    pub app: App,
}

#[async_trait]
impl Service for BluetoothService {
    fn device_type(&self) -> &'static str {
        "sensor"
    }

    async fn run(&self, _name: String) -> Result<()> {
        debug!("Initialize bluetooth services");
        let mut client = BlueZClient::new().unwrap();

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
                                self.app.mqtt.send(format!("{}/rssi", address), format!("{}", rssi).into()).await?;
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
