pub use {
    crate::{config::*, App, spawn, service_interval},
    anyhow::{Context, Error, Result},
    std::sync::Arc,
    tokio::sync::{Mutex, RwLock},
    unstructured::Document,
};

pub type SharedMutex<T> = Arc<Mutex<T>>;
pub type SharedRwLock<T> = Arc<RwLock<T>>;
