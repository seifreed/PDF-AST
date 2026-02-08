use crate::performance::PerformanceConfig;
use log::info;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Progress tracking and callback system
pub struct ProgressTracker {
    config: PerformanceConfig,
    state: Arc<RwLock<ProgressState>>,
    callbacks: Arc<RwLock<Vec<Box<dyn ProgressCallback + Send + Sync>>>>,
}

impl ProgressTracker {
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ProgressState::default())),
            callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a progress callback
    pub fn add_callback<F>(&self, callback: F)
    where
        F: Fn(&ProgressInfo) + Send + Sync + 'static,
    {
        let boxed_callback = Box::new(FunctionCallback(callback));
        self.callbacks.write().push(boxed_callback);
    }

    /// Start tracking progress for a new operation
    pub fn start_operation(&self, name: &str, total_work: u64) -> OperationHandle {
        let operation_id = OperationId::new();
        let operation = Operation {
            id: operation_id,
            name: name.to_string(),
            total_work,
            completed_work: 0,
            start_time: Instant::now(),
            last_update: Instant::now(),
            status: OperationStatus::Running,
            sub_operations: Vec::new(),
            error_message: None,
        };

        self.state.write().operations.push(operation);

        OperationHandle {
            tracker: self.clone(),
            operation_id,
        }
    }

    /// Update progress for an operation
    pub fn update_progress(
        &self,
        operation_id: OperationId,
        completed_work: u64,
        message: Option<&str>,
    ) {
        let mut state = self.state.write();

        if let Some(operation) = state.operations.iter_mut().find(|op| op.id == operation_id) {
            operation.completed_work = completed_work.min(operation.total_work);
            operation.last_update = Instant::now();

            if let Some(msg) = message {
                operation.name = msg.to_string();
            }

            // Check if we should trigger callbacks
            let elapsed_since_last = operation.last_update.duration_since(operation.start_time);
            if elapsed_since_last >= self.config.progress_interval
                || operation.completed_work == operation.total_work
            {
                let progress_info = self.create_progress_info(&state);
                drop(state); // Release lock before calling callbacks

                self.notify_callbacks(&progress_info);
            }
        }
    }

    /// Mark operation as completed
    pub fn complete_operation(&self, operation_id: OperationId) {
        let mut state = self.state.write();

        if let Some(operation) = state.operations.iter_mut().find(|op| op.id == operation_id) {
            operation.status = OperationStatus::Completed;
            operation.completed_work = operation.total_work;

            let progress_info = self.create_progress_info(&state);
            drop(state);

            self.notify_callbacks(&progress_info);
        }
    }

    /// Mark operation as failed
    pub fn fail_operation(&self, operation_id: OperationId, error: &str) {
        let mut state = self.state.write();

        if let Some(operation) = state.operations.iter_mut().find(|op| op.id == operation_id) {
            operation.status = OperationStatus::Failed;
            operation.error_message = Some(error.to_string());

            let progress_info = self.create_progress_info(&state);
            drop(state);

            self.notify_callbacks(&progress_info);
        }
    }

    /// Add a sub-operation
    pub fn add_sub_operation(
        &self,
        parent_id: OperationId,
        name: &str,
        total_work: u64,
    ) -> OperationId {
        let sub_operation_id = OperationId::new();
        let sub_operation = SubOperation {
            id: sub_operation_id,
            name: name.to_string(),
            total_work,
            completed_work: 0,
            status: OperationStatus::Running,
        };

        let mut state = self.state.write();
        if let Some(operation) = state.operations.iter_mut().find(|op| op.id == parent_id) {
            operation.sub_operations.push(sub_operation);
        }

        sub_operation_id
    }

    /// Update sub-operation progress
    pub fn update_sub_operation(
        &self,
        parent_id: OperationId,
        sub_id: OperationId,
        completed_work: u64,
    ) {
        let mut state = self.state.write();

        if let Some(operation) = state.operations.iter_mut().find(|op| op.id == parent_id) {
            if let Some(sub_op) = operation
                .sub_operations
                .iter_mut()
                .find(|sub| sub.id == sub_id)
            {
                sub_op.completed_work = completed_work.min(sub_op.total_work);

                if completed_work >= sub_op.total_work {
                    sub_op.status = OperationStatus::Completed;
                }
            }
        }
    }

    /// Get current progress information
    pub fn get_progress(&self) -> ProgressInfo {
        let state = self.state.read();
        self.create_progress_info(&state)
    }

    /// Clear completed operations
    pub fn cleanup_completed(&self) {
        let mut state = self.state.write();
        state.operations.retain(|op| {
            !matches!(
                op.status,
                OperationStatus::Completed | OperationStatus::Failed
            )
        });
    }

    fn create_progress_info(&self, state: &ProgressState) -> ProgressInfo {
        let mut operations = Vec::new();

        for operation in &state.operations {
            let progress_percent = if operation.total_work > 0 {
                (operation.completed_work as f64 / operation.total_work as f64 * 100.0) as u32
            } else {
                0
            };

            let elapsed = operation.last_update.duration_since(operation.start_time);
            let eta = if operation.completed_work > 0
                && operation.completed_work < operation.total_work
            {
                let rate = operation.completed_work as f64 / elapsed.as_secs_f64();
                let remaining_work = operation.total_work - operation.completed_work;
                Some(Duration::from_secs_f64(remaining_work as f64 / rate))
            } else {
                None
            };

            let sub_operations: Vec<_> = operation
                .sub_operations
                .iter()
                .map(|sub| {
                    let sub_progress = if sub.total_work > 0 {
                        (sub.completed_work as f64 / sub.total_work as f64 * 100.0) as u32
                    } else {
                        0
                    };

                    SubOperationInfo {
                        id: sub.id,
                        name: sub.name.clone(),
                        progress_percent: sub_progress,
                        status: sub.status.clone(),
                    }
                })
                .collect();

            operations.push(OperationInfo {
                id: operation.id,
                name: operation.name.clone(),
                progress_percent,
                completed_work: operation.completed_work,
                total_work: operation.total_work,
                elapsed,
                eta,
                status: operation.status.clone(),
                sub_operations,
                error_message: operation.error_message.clone(),
            });
        }

        ProgressInfo { operations }
    }

    fn notify_callbacks(&self, progress_info: &ProgressInfo) {
        let callbacks = self.callbacks.read();
        for callback in callbacks.iter() {
            callback.on_progress(progress_info);
        }
    }
}

impl Clone for ProgressTracker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            state: Arc::clone(&self.state),
            callbacks: Arc::clone(&self.callbacks),
        }
    }
}

/// Handle for managing a specific operation
pub struct OperationHandle {
    tracker: ProgressTracker,
    operation_id: OperationId,
}

impl OperationHandle {
    pub fn update(&self, completed_work: u64, message: Option<&str>) {
        self.tracker
            .update_progress(self.operation_id, completed_work, message);
    }

    pub fn increment(&self, additional_work: u64, message: Option<&str>) {
        let state = self.tracker.state.read();
        if let Some(operation) = state
            .operations
            .iter()
            .find(|op| op.id == self.operation_id)
        {
            let new_completed = operation.completed_work + additional_work;
            drop(state);
            self.tracker
                .update_progress(self.operation_id, new_completed, message);
        }
    }

    pub fn complete(self) {
        self.tracker.complete_operation(self.operation_id);
    }

    pub fn fail(self, error: &str) {
        self.tracker.fail_operation(self.operation_id, error);
    }

    pub fn add_sub_operation(&self, name: &str, total_work: u64) -> SubOperationHandle {
        let sub_id = self
            .tracker
            .add_sub_operation(self.operation_id, name, total_work);
        SubOperationHandle {
            tracker: self.tracker.clone(),
            parent_id: self.operation_id,
            sub_id,
        }
    }

    pub fn get_id(&self) -> OperationId {
        self.operation_id
    }
}

/// Handle for managing sub-operations
pub struct SubOperationHandle {
    tracker: ProgressTracker,
    parent_id: OperationId,
    sub_id: OperationId,
}

impl SubOperationHandle {
    pub fn update(&self, completed_work: u64) {
        self.tracker
            .update_sub_operation(self.parent_id, self.sub_id, completed_work);
    }

    pub fn complete(self) {
        let state = self.tracker.state.read();
        if let Some(operation) = state.operations.iter().find(|op| op.id == self.parent_id) {
            if let Some(sub_op) = operation
                .sub_operations
                .iter()
                .find(|sub| sub.id == self.sub_id)
            {
                let total = sub_op.total_work;
                drop(state);
                self.tracker
                    .update_sub_operation(self.parent_id, self.sub_id, total);
            }
        }
    }
}

/// Trait for progress callbacks
pub trait ProgressCallback {
    fn on_progress(&self, progress: &ProgressInfo);
}

struct FunctionCallback<F>(F);

impl<F> ProgressCallback for FunctionCallback<F>
where
    F: Fn(&ProgressInfo) + Send + Sync,
{
    fn on_progress(&self, progress: &ProgressInfo) {
        (self.0)(progress);
    }
}

/// Built-in progress callbacks
pub struct ConsoleProgressCallback {
    last_print: RwLock<Instant>,
    min_interval: Duration,
}

impl ConsoleProgressCallback {
    pub fn new(min_interval: Duration) -> Self {
        Self {
            last_print: RwLock::new(Instant::now()),
            min_interval,
        }
    }
}

impl ProgressCallback for ConsoleProgressCallback {
    fn on_progress(&self, progress: &ProgressInfo) {
        let now = Instant::now();
        let mut last_print = self.last_print.write();

        if now.duration_since(*last_print) >= self.min_interval {
            *last_print = now;

            for op in &progress.operations {
                if matches!(op.status, OperationStatus::Running) {
                    let eta_str = op
                        .eta
                        .map(|eta| format!(" ETA: {:?}", eta))
                        .unwrap_or_default();

                    info!("[{}%] {}{}", op.progress_percent, op.name, eta_str);

                    for sub_op in &op.sub_operations {
                        if matches!(sub_op.status, OperationStatus::Running) {
                            info!("  └─ [{}%] {}", sub_op.progress_percent, sub_op.name);
                        }
                    }
                }
            }
        }
    }
}

// Data structures
#[derive(Debug, Default)]
struct ProgressState {
    operations: Vec<Operation>,
}

#[derive(Debug)]
struct Operation {
    id: OperationId,
    name: String,
    total_work: u64,
    completed_work: u64,
    start_time: Instant,
    last_update: Instant,
    status: OperationStatus,
    sub_operations: Vec<SubOperation>,
    error_message: Option<String>,
}

#[derive(Debug)]
struct SubOperation {
    id: OperationId,
    name: String,
    total_work: u64,
    completed_work: u64,
    status: OperationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationId(u64);

impl OperationId {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperationStatus {
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub operations: Vec<OperationInfo>,
}

#[derive(Debug, Clone)]
pub struct OperationInfo {
    pub id: OperationId,
    pub name: String,
    pub progress_percent: u32,
    pub completed_work: u64,
    pub total_work: u64,
    pub elapsed: Duration,
    pub eta: Option<Duration>,
    pub status: OperationStatus,
    pub sub_operations: Vec<SubOperationInfo>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SubOperationInfo {
    pub id: OperationId,
    pub name: String,
    pub progress_percent: u32,
    pub status: OperationStatus,
}

/// Convenience macros for common progress patterns
#[macro_export]
macro_rules! with_progress {
    ($tracker:expr, $name:expr, $total:expr, $code:block) => {{
        let handle = $tracker.start_operation($name, $total);
        let result = $code;
        handle.complete();
        result
    }};
}

#[macro_export]
macro_rules! progress_loop {
    ($tracker:expr, $name:expr, $items:expr, |$item:ident, $handle:ident| $code:block) => {{
        let total = $items.len() as u64;
        let $handle = $tracker.start_operation($name, total);
        let mut results = Vec::new();

        for (i, $item) in $items.into_iter().enumerate() {
            $handle.update(i as u64, None);
            results.push($code);
        }

        $handle.complete();
        results
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_progress_tracker_basic() {
        let config = PerformanceConfig::default();
        let tracker = ProgressTracker::new(config);

        let handle = tracker.start_operation("Test Operation", 100);
        handle.update(50, Some("Halfway done"));

        let progress = tracker.get_progress();
        assert_eq!(progress.operations.len(), 1);
        assert_eq!(progress.operations[0].progress_percent, 50);

        handle.complete();

        let progress = tracker.get_progress();
        assert_eq!(progress.operations[0].progress_percent, 100);
        assert_eq!(progress.operations[0].status, OperationStatus::Completed);
    }

    #[test]
    fn test_sub_operations() {
        let config = PerformanceConfig::default();
        let tracker = ProgressTracker::new(config);

        let handle = tracker.start_operation("Parent", 100);
        let sub_handle = handle.add_sub_operation("Child", 50);

        sub_handle.update(25);
        let progress = tracker.get_progress();

        assert_eq!(progress.operations[0].sub_operations.len(), 1);
        assert_eq!(
            progress.operations[0].sub_operations[0].progress_percent,
            50
        );

        sub_handle.complete();
        handle.complete();
    }

    #[test]
    fn test_progress_callbacks() {
        let config = PerformanceConfig {
            progress_interval: Duration::from_nanos(1), // Very short interval to ensure callbacks fire
            ..Default::default()
        };
        let tracker = ProgressTracker::new(config);

        let callback_count = Arc::new(AtomicUsize::new(0));
        let callback_count_clone = Arc::clone(&callback_count);

        tracker.add_callback(move |_progress| {
            callback_count_clone.fetch_add(1, Ordering::Relaxed);
        });

        let handle = tracker.start_operation("Test", 10);
        handle.update(5, None);
        handle.complete();

        // Should have been called at least twice (update + complete)
        assert!(callback_count.load(Ordering::Relaxed) >= 2);
    }

    #[test]
    fn test_progress_macros() {
        let config = PerformanceConfig::default();
        let tracker = ProgressTracker::new(config);

        let result = with_progress!(tracker, "Macro Test", 1, { "test result" });

        assert_eq!(result, "test result");

        let items = vec![1, 2, 3, 4, 5];
        let results = progress_loop!(tracker, "Loop Test", items, |item, handle| { item * 2 });

        assert_eq!(results, vec![2, 4, 6, 8, 10]);
    }
}
