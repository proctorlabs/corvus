use {
    crate::{config::TriggerConfiguration, services::Services, *},
    interval::IntervalTrigger,
    on_start::OnStartTrigger,
};

mod interval;
mod on_start;

pub trait Trigger {
    fn init(&self, service: Services) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum Triggers {
    Interval { trigger: IntervalTrigger },
    OnStart { trigger: OnStartTrigger },
    MQTT { trigger: IntervalTrigger },
}

impl Triggers {
    pub fn new(cfg: Arc<TriggerConfiguration>) -> Self {
        match &*cfg {
            TriggerConfiguration::Interval { interval } => Triggers::Interval {
                trigger: IntervalTrigger::new(*interval),
            },
            TriggerConfiguration::MQTT { .. } => unimplemented!(),
            TriggerConfiguration::Start { .. } => Triggers::OnStart {
                trigger: OnStartTrigger::new(),
            },
        }
    }

    pub fn init(&self, service: Services) -> Result<()> {
        match self {
            Triggers::Interval { trigger } => trigger.init(service),
            Triggers::MQTT { trigger } => trigger.init(service),
            Triggers::OnStart { trigger } => trigger.init(service),
        }
    }
}
