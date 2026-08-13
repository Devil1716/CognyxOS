use cognyx_proto::cognyx::services::runtime::v1::{RuntimeEvent, RuntimeType};
use tracing::info;

pub struct RuntimeEventPublisher;

impl RuntimeEventPublisher {
    pub fn publish_event(
        runtime_id: &str,
        r_type: RuntimeType,
        event_type: &str,
        message: &str,
    ) -> RuntimeEvent {
        info!(
            "RUNTIME EVENT [{}] (id: {}): {}",
            event_type, runtime_id, message
        );

        RuntimeEvent {
            event_id: format!("evt-{}", uuid::Uuid::now_v7()),
            runtime_id: runtime_id.to_string(),
            runtime_type: r_type as i32,
            event_type: event_type.to_string(),
            message: message.to_string(),
            timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        }
    }
}
