use {
    super::*,
    crate::mqtt::{DeviceType, HassDiscoveryPayload},
    serde::{Deserialize, Serialize},
    std::process::Stdio,
    tokio::process::Command,
    unstructured::Document,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct CommandPayload {
    status: i32,
    stdout: String,
    stderr: String,
}

#[derive(Clone, Debug)]
pub struct CommandService {
    pub app: App,
    pub command: String,
    pub args: Vec<String>,
}

#[async_trait]
impl Service for CommandService {
    fn device_type(&self) -> &'static str {
        "sensor"
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

        self.app
            .mqtt
            .send(
                name.clone(),
                CommandPayload {
                    status: output.status.code().unwrap_or_default(),
                    stdout: stdout.trim().into(),
                    stderr: stderr.trim().into(),
                }
                .into(),
            )
            .await?;

        let cfg = self.clone();
        let (name, app) = (name, cfg.app);
        service_interval!((30): {
            let devinfo = HassDiscoveryPayload::default();
            app.mqtt.add_device(DeviceType::Sensor, name.clone(), devinfo).await?;
        });
        Ok(())
    }
}

impl From<CommandPayload> for Document {
    fn from(m: CommandPayload) -> Self {
        Document::new(m).unwrap_or_default()
    }
}
