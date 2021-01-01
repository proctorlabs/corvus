pub use crate::{
    config::*, data_structures::*, device_registry::*, mqtt::MQTTService, util::*, App,
};
pub use anyhow::{Context, Error, Result};
pub use std::sync::Arc;
pub use tokio::sync::{Mutex, RwLock};
pub use unstructured::Document;

pub type SharedMutex<T> = Arc<Mutex<T>>;
pub type SharedRwLock<T> = Arc<RwLock<T>>;
