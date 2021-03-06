use super::*;

#[derive(Debug, Clone, Default)]
pub struct OnStartTrigger {}

impl OnStartTrigger {
    pub fn new() -> Self {
        OnStartTrigger::default()
    }
}

impl Trigger for OnStartTrigger {
    fn init(&self, service: Plugins) -> Result<()> {
        info!("Starting service '{}'", service.name());
        start_service(
            Duration::from_secs(2),
            format!("Startup trigger for {}", service.name()),
            true,
            false,
            move || {
                let service = service.clone();
                async move { service.run().await }
            },
        )?;
        Ok(())
    }
}
