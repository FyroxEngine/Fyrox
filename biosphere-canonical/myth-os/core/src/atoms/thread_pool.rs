// CORE-ATOM-06: Thread Orchestrator — Tokio task pools
use tokio::task::JoinHandle;
use tracing::info;

pub struct ThreadOrchestrator {
    pub worker_count: usize,
}

impl ThreadOrchestrator {
    pub fn new(worker_count: usize) -> Self {
        info!(workers = worker_count, "ThreadOrchestrator initialized");
        Self { worker_count }
    }

    /// Spawn a named Tokio task.
    pub fn spawn<F, Fut>(&self, name: &'static str, f: F) -> JoinHandle<()>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        info!(task = name, "Spawning task");
        tokio::spawn(f())
    }
}
