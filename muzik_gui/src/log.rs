use std::collections::BTreeMap;

use crossbeam_channel::Sender;
use tracing::{field::Field, Level};
use tracing_subscriber::Layer;

pub struct StatusLayer {
    sender: Sender<GuiEvent>,
}

impl StatusLayer {
    pub fn new(tx: Sender<GuiEvent>) -> Self {
        (Self { sender: tx })
    }
}

impl<S> Layer<S> for StatusLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // send this to
        let mut fields = BTreeMap::new();
        let mut visitor = CollectVisitor(&mut fields);
        event.record(&mut visitor);

        let level = event.metadata().level();
        let target = event.metadata().target();
        let name = event.metadata().name();

        let gui_event = GuiEvent {
            level: level.clone(),
            target: target.to_string(),
            span_name: name.to_string(),
            fields,
        };
        self.sender.send(gui_event).expect("receiving end works");
    }
}

#[derive(Clone, Debug)]
pub struct GuiEvent {
    pub level: Level,
    pub target: String,
    pub span_name: String,

    pub fields: BTreeMap<String, String>,
}

impl GuiEvent {
    pub fn get_message(&self) -> String {
        self.fields
            .get("message")
            .unwrap_or(&"no message recorded".to_string())
            .to_string()
    }
}

struct CollectVisitor<'a>(&'a mut BTreeMap<String, String>);
impl<'a> tracing::field::Visit for CollectVisitor<'a> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{:?}", value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.record_debug(field, &value)
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_debug(field, &value)
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record_debug(field, &value)
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.record_debug(field, &value)
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.record_debug(field, &value)
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record_debug(field, &value)
    }
}
