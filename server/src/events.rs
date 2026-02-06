use chrono::{DateTime, Utc};
use tokio::sync::broadcast;
use uuid::Uuid;

/// Represents a job state transition event
#[derive(Clone, Debug)]
pub struct JobEvent {
    pub job_id: Uuid,
    pub previous_state: String,
    pub new_state: String,
    pub verification_status: String,
    pub timestamp: DateTime<Utc>,
}

/// Broadcast channel for job events, enabling real-time streaming
#[derive(Clone)]
pub struct JobEventBus {
    sender: broadcast::Sender<JobEvent>,
}

impl JobEventBus {
    /// Create a new event bus with the specified channel capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish a job event to all subscribers
    pub fn publish(&self, event: JobEvent) {
        // Ignore error if no receivers are listening
        let _ = self.sender.send(event);
    }

    /// Subscribe to job events
    pub fn subscribe(&self) -> broadcast::Receiver<JobEvent> {
        self.sender.subscribe()
    }
}
