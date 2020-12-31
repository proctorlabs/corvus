#[macro_use]
extern crate clap;

#[macro_use]
extern crate derive_more;

#[macro_use]
extern crate log;

mod app;
mod config;
mod data_structures;
mod device_registry;
mod mqtt;
mod plugins;
mod prelude;
mod triggers;
mod util;

pub use app::App;
pub use prelude::*;
use std::time::Duration;

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<()> {
    App::new().await?.start().await
}
