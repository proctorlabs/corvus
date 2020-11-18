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
    fn init(&self, service: Services) -> Result<()> {
        info!(
            "Starting service '{}' with interval trigger every {} seconds.",
            service.name(),
            self.interval
        );
        let interval = self.interval;
        service_interval!((interval) : {
            service.run().await?;
        });
        Ok(())
    }
}
