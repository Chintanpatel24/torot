use crate::core::types::Finding;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum AppEvent {
    Line {
        tool: String,
        line: String,
        kind: String,
        severity: Option<String>,
    },
    Finding(Finding),
    ScanComplete {
        report_path: Option<String>,
    },
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<AppEvent>,
}

impl EventBus {
    pub fn new() -> (Self, broadcast::Receiver<AppEvent>) {
        let (tx, rx) = broadcast::channel(2048);
        (Self { tx }, rx)
    }

    pub fn emit(&self, event: AppEvent) {
        if let Err(e) = self.tx.send(event) {
            tracing::debug!("event bus send lagged: {e}");
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.tx.subscribe()
    }
}
