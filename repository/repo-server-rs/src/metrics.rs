// Metrics and monitoring capabilities
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Metrics collection for the repo server
#[derive(Debug, Clone)]
pub struct Metrics {
    pub messages_processed: u64,
    pub messages_failed: u64,
    pub version_requests: u64,
    pub delete_requests: u64,
    pub size_requests: u64,
    pub path_create_requests: u64,
    pub path_delete_requests: u64,
    pub total_bytes_processed: u64,
    pub average_processing_time: Duration,
    pub uptime: Duration,
    pub worker_threads_active: u32,
    pub last_activity: Option<Instant>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            messages_processed: 0,
            messages_failed: 0,
            version_requests: 0,
            delete_requests: 0,
            size_requests: 0,
            path_create_requests: 0,
            path_delete_requests: 0,
            total_bytes_processed: 0,
            average_processing_time: Duration::ZERO,
            uptime: Duration::ZERO,
            worker_threads_active: 0,
            last_activity: None,
        }
    }
}

/// Thread-safe metrics collector
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    metrics: Arc<Mutex<Metrics>>,
    start_time: Instant,
    processing_times: Arc<Mutex<Vec<Duration>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(Metrics::default())),
            start_time: Instant::now(),
            processing_times: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn record_message_processed(&self, message_type: u16, bytes: usize, processing_time: Duration) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.messages_processed += 1;
        metrics.total_bytes_processed += bytes as u64;
        metrics.last_activity = Some(Instant::now());
        metrics.uptime = self.start_time.elapsed();

        // Update specific message type counters
        match message_type {
            1 => metrics.version_requests += 1, // VERSION_REQUEST
            17930 => metrics.delete_requests += 1, // REPO_DATA_DELETE_REQUEST
            13322 => metrics.size_requests += 1, // REPO_DATA_GET_SIZE_REQUEST
            13578 => metrics.path_create_requests += 1, // REPO_PATH_CREATE_REQUEST
            1004 => metrics.path_delete_requests += 1, // REPO_PATH_DELETE_REQUEST
            _ => {}
        }

        // Update average processing time
        {
            let mut times = self.processing_times.lock().unwrap();
            times.push(processing_time);
            if times.len() > 1000 {
                times.drain(0..100); // Keep only last 900 entries
            }
            
            let total_time: Duration = times.iter().sum();
            metrics.average_processing_time = total_time / times.len().max(1) as u32;
        }
    }

    pub fn record_message_failed(&self, _message_type: u16) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.messages_failed += 1;
        metrics.last_activity = Some(Instant::now());
        metrics.uptime = self.start_time.elapsed();
    }

    pub fn set_worker_threads_active(&self, count: u32) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.worker_threads_active = count;
    }

    pub fn get_metrics(&self) -> Metrics {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.uptime = self.start_time.elapsed();
        metrics.clone()
    }

    pub fn get_metrics_summary(&self) -> String {
        let metrics = self.get_metrics();
        format!(
            "Metrics Summary:\n\
            \tMessages Processed: {}\n\
            \tMessages Failed: {}\n\
            \tVersion Requests: {}\n\
            \tDelete Requests: {}\n\
            \tSize Requests: {}\n\
            \tPath Create Requests: {}\n\
            \tPath Delete Requests: {}\n\
            \tTotal Bytes Processed: {}\n\
            \tAverage Processing Time: {:?}\n\
            \tUptime: {:?}\n\
            \tActive Worker Threads: {}\n\
            \tLast Activity: {:?}",
            metrics.messages_processed,
            metrics.messages_failed,
            metrics.version_requests,
            metrics.delete_requests,
            metrics.size_requests,
            metrics.path_create_requests,
            metrics.path_delete_requests,
            metrics.total_bytes_processed,
            metrics.average_processing_time,
            metrics.uptime,
            metrics.worker_threads_active,
            metrics.last_activity.map(|t| t.elapsed()).unwrap_or(Duration::ZERO)
        )
    }

    pub fn reset(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        *metrics = Metrics::default();
        self.processing_times.lock().unwrap().clear();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance monitoring utilities
pub struct PerformanceMonitor {
    start_time: Instant,
    operation_name: String,
}

impl PerformanceMonitor {
    pub fn start(operation_name: &str) -> Self {
        Self {
            start_time: Instant::now(),
            operation_name: operation_name.to_string(),
        }
    }

    pub fn finish(self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn finish_with_logging(self, _metrics: &MetricsCollector) -> Duration {
        let duration = self.finish();
        // Could add logging here if needed
        duration
    }
}

/// Health check utilities
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub issues: Vec<String>,
    pub uptime: Duration,
    pub last_activity: Option<Duration>,
}

impl MetricsCollector {
    pub fn health_check(&self) -> HealthStatus {
        let metrics = self.get_metrics();
        let mut issues = Vec::new();

        // Check if server has been idle for too long (5 minutes)
        if let Some(last_activity) = metrics.last_activity {
            if last_activity.elapsed() > Duration::from_secs(300) {
                issues.push("Server has been idle for more than 5 minutes".to_string());
            }
        }

        // Check if error rate is too high (>10%)
        if metrics.messages_processed > 0 {
            let error_rate = (metrics.messages_failed as f64 / metrics.messages_processed as f64) * 100.0;
            if error_rate > 10.0 {
                issues.push(format!("High error rate: {:.1}%", error_rate));
            }
        }

        // Check if no worker threads are active
        if metrics.worker_threads_active == 0 {
            issues.push("No worker threads are active".to_string());
        }

        let is_healthy = issues.is_empty();

        HealthStatus {
            is_healthy,
            issues,
            uptime: metrics.uptime,
            last_activity: metrics.last_activity.map(|t| t.elapsed()),
        }
    }
}
