pub struct GraphScheduler;

#[allow(dead_code)]
pub struct MultiAgentScheduler {
    base_scheduler: GraphScheduler,
}

impl MultiAgentScheduler {
    pub fn new() -> Self {
        Self { base_scheduler: GraphScheduler }
    }
    
    pub fn schedule_task(&self, _task_id: &str) {}
}
