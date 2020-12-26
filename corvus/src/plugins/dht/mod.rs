use dht22::*;
use linux_embedded_hal::{gpio_cdev::*, CdevPin};
use nix::unistd::close;
use std::{
    env, fmt,
    os::unix::io::AsRawFd,
    time::{Duration, Instant},
};

mod dht22;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let chip = &args[1];
    let offset: u32 = args[2].parse()?;
    let mut dht = DHT::new(chip, offset)?;
    loop {
        let reading = dht.get_reading();
        match reading {
            Ok(val) => {
                println!("{:?}", val);
                std::thread::sleep(Duration::from_secs(12));
            }
            Err(e) => {
                println!("{:?}", e);
                // std::thread::sleep(Duration::from_secs(5));
            }
        }
        // std::thread::sleep(Duration::from_secs(12));
    }
    unreachable!();
    Ok(())
}

use super::*;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;
use unstructured::Document;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct DHTPayload {
    humidity:    f32,
    temperature: f32,
}

// #[derive(Debug)]
pub struct DHTPlugin {
    dht: DHT,
}

impl DHTPlugin {
    pub fn new(device: String, channel: u32) -> Result<Self> {
        Ok(DHTPlugin {
            dht: DHT::new(&device, channel).unwrap(),
        })
    }
}

#[async_trait]
impl Plugin for DHTPlugin {
    async fn leader_heartbeat(&self, _: String, _: ClusterNodes) -> Result<()> {
        Ok(())
    }

    async fn heartbeat(&self, name: String) -> Result<()> {
        Ok(())
    }

    async fn run(&self, name: String) -> Result<()> {
        Ok(())
    }
}

impl From<DHTPayload> for Document {
    fn from(m: DHTPayload) -> Self {
        Document::new(m).unwrap_or_default()
    }
}
