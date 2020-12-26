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
        spawn! {
            service.run().await?;
        };
        Ok(())
    }
}
