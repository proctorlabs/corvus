use super::*;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;
use unstructured::Document;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct CommandPayload {
    status: i32,
    stdout: String,
    stderr: String,
}

#[derive(Clone, Debug)]
pub struct CommandPlugin {
    mqtt:     MQTTService,
    registry: DeviceRegistry,
    command:  String,
    args:     Vec<String>,
}

impl CommandPlugin {
    pub fn new(
        mqtt: MQTTService,
        registry: DeviceRegistry,
        command: String,
        args: Vec<String>,
    ) -> Self {
        Self {
            mqtt,
            registry,
            command,
            args,
        }
    }
}

#[async_trait]
impl Plugin for CommandPlugin {
    async fn leader_heartbeat(&self, _: String, _: ClusterNodes) -> Result<()> {
        Ok(())
    }

    async fn heartbeat(&self, name: String) -> Result<()> {
        self.registry
            .register(
                self.registry
                    .new_device(name, DeviceType::Sensor(SensorDeviceClass::None))
                    .build(),
            )
            .await
    }

    async fn run(&self, name: String) -> Result<()> {
        let output = Command::new(self.command.to_string())
            .args(&*self.args.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;
        let stdout = String::from_utf8(output.stdout).unwrap_or_default();
        let stderr = String::from_utf8(output.stderr).unwrap_or_default();
        let d = self.registry.get_by_name(&name).await;
        let update = DeviceUpdate {
            device: d,
            value:  stdout.trim().into(),
            attr:   CommandPayload {
                status: output.status.code().unwrap_or_default(),
                stdout: stdout.trim().into(),
                stderr: stderr.trim().into(),
            }
            .into(),
        };

        self.mqtt.update_device(&update).await
    }
}

impl From<CommandPayload> for Document {
    fn from(m: CommandPayload) -> Self {
        Document::new(m).unwrap_or_default()
    }
}
