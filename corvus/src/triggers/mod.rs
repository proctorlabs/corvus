use crate::{config::TriggerConfiguration, plugins::Plugins, *};
use interval::IntervalTrigger;
use on_start::OnStartTrigger;

mod interval;
mod on_start;

pub trait Trigger {
    fn init(&self, service: Plugins) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum Triggers {
    Interval(IntervalTrigger),
    OnStart(OnStartTrigger),
    MQTT(IntervalTrigger),
}

impl Triggers {
    pub fn new(cfg: Arc<TriggerConfiguration>) -> Self {
        match &*cfg {
            TriggerConfiguration::Interval { interval } => {
                Triggers::Interval(IntervalTrigger::new(*interval))
            }
            TriggerConfiguration::MQTT { .. } => unimplemented!(),
            TriggerConfiguration::Start { .. } => Triggers::OnStart(OnStartTrigger::new()),
        }
    }

    pub fn init(&self, service: Plugins) -> Result<()> {
        match self {
            Triggers::Interval(trigger) => trigger.init(service),
            Triggers::MQTT(trigger) => trigger.init(service),
            Triggers::OnStart(trigger) => trigger.init(service),
        }
    }
}
