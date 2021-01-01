use crate::prelude::*;
pub use async_trait::async_trait;
use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone)]
pub struct ServiceData<T, F>
where
    T: Clone + Send + Sync + Fn() -> F + Send + Sync + 'static,
    F: Clone + Send + Sync + std::future::Future<Output = Result<()>> + Sync + Send,
{
    pub name:              String,
    pub start_immediately: bool,
    pub add_jitter:        bool,
    pub duration:          Duration,
    pub service_method:    T,
}

#[async_trait]
pub trait StaticService: Send + Sync + Clone {
    const NAME: &'static str;
    const START_IMMEDIATELY: bool;
    const ADD_JITTER: bool;
    const DURATION: Duration;
    async fn exec_service(zelf: Self) -> Result<()>;
}

#[async_trait]
impl<T, F> Service for ServiceData<T, F>
where
    T: Clone + Send + Sync + Fn() -> F + Send + Sync + 'static,
    F: Clone + Send + Sync + std::future::Future<Output = Result<()>> + Sync + Send,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn start_immediately(&self) -> bool {
        self.start_immediately
    }

    fn add_jitter(&self) -> bool {
        self.add_jitter
    }

    fn duration(&self) -> Duration {
        self.duration
    }

    async fn exec_service(self) -> Result<()> {
        let fut = (self.service_method)();
        fut.await
    }
}

#[async_trait]
impl<T: StaticService> Service for T {
    fn name(&self) -> &str {
        T::NAME
    }

    fn start_immediately(&self) -> bool {
        T::START_IMMEDIATELY
    }

    fn add_jitter(&self) -> bool {
        T::ADD_JITTER
    }

    fn duration(&self) -> Duration {
        T::DURATION
    }

    async fn exec_service(self) -> Result<()> {
        T::exec_service(self.clone()).await
    }
}

#[async_trait]
pub trait Service: Send + Sync + Clone {
    fn name(&self) -> &str;
    fn start_immediately(&self) -> bool;
    fn add_jitter(&self) -> bool;
    fn duration(&self) -> Duration;
    async fn exec_service(self) -> Result<()>;

    fn start_service(&self) -> Result<()>
    where
        Self: 'static,
    {
        let zelf = self.clone();
        start_service(
            self.duration(),
            self.name().into(),
            self.start_immediately(),
            self.add_jitter(),
            move || {
                let zelf = zelf.clone();
                async move { zelf.clone().exec_service().await }
            },
        )?;
        Ok(())
    }
}

pub fn start_service<T, F>(
    dur: Duration,
    name: String,
    immediate: bool,
    jitter: bool,
    f: T,
) -> Result<()>
where
    T: Fn() -> F + Send + 'static,
    F: std::future::Future<Output = Result<()>> + Send,
{
    tokio::spawn(async move {
        if !immediate {
            sleep(dur).await;
        }
        info!("Starting service {}", name);
        loop {
            let dur = if jitter {
                dur + Duration::from_millis(rand::thread_rng().gen_range(0..2000))
                    - Duration::from_millis(1000)
            } else {
                dur
            };
            let tsk = f();
            debug!("Running {}", name);
            match tsk.await {
                Ok(_) => sleep(dur).await,
                Err(e) => {
                    error!("Task {} failure! {:?}", name, e);
                    sleep(dur).await;
                }
            }
        }
    });
    Ok(())
}
