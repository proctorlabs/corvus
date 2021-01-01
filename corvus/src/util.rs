use crate::prelude::*;
use std::time::Duration;

pub fn start_service<T, F>(dur: Duration, name: String, immediate: bool, f: T) -> Result<()>
where
    T: Fn() -> F + Send + 'static,
    F: std::future::Future<Output = Result<()>> + Send,
{
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(dur);
        if !immediate {
            interval.tick().await;
        }
        info!("Starting service {}", name);
        loop {
            interval.tick().await;
            let tsk = f();
            debug!("Running {}", name);
            match tsk.await {
                Ok(_) => continue,
                Err(e) => error!("Task {} failure! {:?}", name, e),
            }
        }
    });
    Ok(())
}
