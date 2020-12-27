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
    pub app:     App,
    pub command: String,
    pub args:    Vec<String>,
}

impl CommandPlugin {
    pub fn new(app: App, command: String, args: Vec<String>) -> Self {
        Self { app, command, args }
    }
}

#[async_trait]
impl Plugin for CommandPlugin {
    async fn leader_heartbeat(&self, _: String, _: ClusterNodes) -> Result<()> {
        Ok(())
    }

    async fn heartbeat(&self, name: String) -> Result<()> {
        self.app
            .mqtt
            .add_device(DeviceInfo {
                name,
                typ: DeviceType::Sensor,
                is_cluster_device: false,
            })
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
        let update = DeviceUpdate {
            name,
            value: stdout.trim().into(),
            attr: Some(
                CommandPayload {
                    status: output.status.code().unwrap_or_default(),
                    stdout: stdout.trim().into(),
                    stderr: stderr.trim().into(),
                }
                .into(),
            ),
            is_cluster_device: false,
        };

        self.app.mqtt.update_device(&update).await
    }
}

impl From<CommandPayload> for Document {
    fn from(m: CommandPayload) -> Self {
        Document::new(m).unwrap_or_default()
    }
}
