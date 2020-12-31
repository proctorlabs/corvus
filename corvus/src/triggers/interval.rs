use super::*;

#[derive(Debug, Clone)]
pub struct IntervalTrigger {
    interval: u64,
}

impl IntervalTrigger {
    pub fn new(interval: u64) -> Self {
        IntervalTrigger { interval }
    }
}

impl Trigger for IntervalTrigger {
    fn init(&self, service: Plugins) -> Result<()> {
        info!(
            "Starting service '{}' with interval trigger every {} seconds.",
            service.name(),
            self.interval
        );
        start_service(Duration::from_secs(self.interval), move || {
            let service = service.clone();
            async move { service.run().await }
        })?;
        Ok(())
    }
}
