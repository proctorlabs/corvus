use crate::prelude::*;
use std::time::Duration;

pub fn start_service<T, F>(dur: Duration, f: T) -> Result<()>
where
    T: Fn() -> F + Send + 'static,
    F: std::future::Future<Output = Result<()>> + Send,
{
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(dur);
        interval.tick().await;
        loop {
            interval.tick().await;
            let tsk = f();
            match tsk.await {
                Ok(_) => continue,
                Err(e) => error!("Task failure! {:?}", e),
            }
        }
    });
    Ok(())
}
