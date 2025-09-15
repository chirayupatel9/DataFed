// Performance benchmarking and optimization utilities
use std::time::{Duration, Instant};
use std::sync::Arc;
use crate::metrics::MetricsCollector;

/// Benchmark results for a specific operation
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub operation_name: String,
    pub iterations: u32,
    pub total_time: Duration,
    pub average_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
    pub operations_per_second: f64,
}

/// Benchmark suite for testing repo server performance
pub struct BenchmarkSuite {
    metrics: Arc<MetricsCollector>,
}

impl BenchmarkSuite {
    pub fn new(metrics: Arc<MetricsCollector>) -> Self {
        Self { metrics }
    }

    /// Benchmark message serialization performance
    pub fn benchmark_message_serialization(&self, iterations: u32) -> BenchmarkResult {
        let mut times = Vec::new();
        let start = Instant::now();

        for i in 0..iterations {
            let iter_start = Instant::now();
            
            // Create a test message
            let test_message = self.create_test_message(i);
            
            // Serialize it
            let _serialized = test_message.serialize().unwrap();
            
            let iter_time = iter_start.elapsed();
            times.push(iter_time);
        }

        let total_time = start.elapsed();
        self.calculate_benchmark_result("message_serialization", iterations, times, total_time)
    }

    /// Benchmark message deserialization performance
    pub fn benchmark_message_deserialization(&self, iterations: u32) -> BenchmarkResult {
        let mut times = Vec::new();
        let start = Instant::now();

        // Pre-serialize test messages
        let test_messages: Vec<Vec<u8>> = (0..iterations)
            .map(|i| self.create_test_message(i).serialize().unwrap())
            .collect();

        for serialized in test_messages {
            let iter_start = Instant::now();
            
            // Deserialize it
            let _message = crate::message::Message::deserialize(&serialized).unwrap();
            
            let iter_time = iter_start.elapsed();
            times.push(iter_time);
        }

        let total_time = start.elapsed();
        self.calculate_benchmark_result("message_deserialization", iterations, times, total_time)
    }

    /// Benchmark path sanitization performance
    pub fn benchmark_path_sanitization(&self, iterations: u32) -> BenchmarkResult {
        use crate::path_utils::PathSanitizer;

        let temp_dir = std::env::temp_dir().join("benchmark_test");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let sanitizer = PathSanitizer::new(&temp_dir);

        let test_paths = vec![
            "test/file1.txt",
            "test/file2.txt",
            "test/subdir/file3.txt",
            "/absolute/path/file4.txt",
            "relative/path/file5.txt",
        ];

        let mut times = Vec::new();
        let start = Instant::now();

        for i in 0..iterations {
            let path = test_paths[i as usize % test_paths.len()];
            let iter_start = Instant::now();
            
            // Sanitize the path
            let _result = sanitizer.sanitize_path(path).unwrap();
            
            let iter_time = iter_start.elapsed();
            times.push(iter_time);
        }

        let total_time = start.elapsed();
        self.calculate_benchmark_result("path_sanitization", iterations, times, total_time)
    }

    /// Benchmark worker message processing
    pub fn benchmark_worker_processing(&self, iterations: u32) -> BenchmarkResult {
        let mut times = Vec::new();
        let start = Instant::now();

        for i in 0..iterations {
            let iter_start = Instant::now();
            
            // Simulate message processing
            let test_message = self.create_test_message(i);
            let _serialized = test_message.serialize().unwrap();
            
            let iter_time = iter_start.elapsed();
            times.push(iter_time);
        }

        let total_time = start.elapsed();
        self.calculate_benchmark_result("worker_processing", iterations, times, total_time)
    }

    /// Run all benchmarks and return results
    pub fn run_all_benchmarks(&self, iterations: u32) -> Vec<BenchmarkResult> {
        println!("Running performance benchmarks with {} iterations each...", iterations);
        
        let mut results = Vec::new();
        
        // Message serialization benchmark
        println!("Benchmarking message serialization...");
        results.push(self.benchmark_message_serialization(iterations));
        
        // Message deserialization benchmark
        println!("Benchmarking message deserialization...");
        results.push(self.benchmark_message_deserialization(iterations));
        
        // Path sanitization benchmark
        println!("Benchmarking path sanitization...");
        results.push(self.benchmark_path_sanitization(iterations));
        
        // Worker processing benchmark
        println!("Benchmarking worker processing...");
        results.push(self.benchmark_worker_processing(iterations));
        
        results
    }

    /// Print benchmark results in a formatted table
    pub fn print_benchmark_results(&self, results: &[BenchmarkResult]) {
        println!("\n=== Performance Benchmark Results ===");
        println!("{:<25} {:<10} {:<12} {:<12} {:<12} {:<12} {:<12}", 
                 "Operation", "Iterations", "Total (ms)", "Avg (μs)", "Min (μs)", "Max (μs)", "Ops/sec");
        println!("{}", "-".repeat(100));
        
        for result in results {
            println!("{:<25} {:<10} {:<12.2} {:<12.2} {:<12.2} {:<12.2} {:<12.0}",
                     result.operation_name,
                     result.iterations,
                     result.total_time.as_secs_f64() * 1000.0,
                     result.average_time.as_micros() as f64,
                     result.min_time.as_micros() as f64,
                     result.max_time.as_micros() as f64,
                     result.operations_per_second);
        }
        println!();
    }

    fn create_test_message(&self, id: u32) -> crate::message::Message {
        use crate::message::*;
        use crate::proto::*;

        let payload = match id % 4 {
            0 => MessagePayload::VersionRequest(VersionRequest),
            1 => MessagePayload::Nack(NackReply {
                err_code: -1,
                err_msg: format!("Test error {}", id),
            }),
            2 => MessagePayload::Ack(AckReply),
            _ => MessagePayload::Empty,
        };

        let mut message = Message::new(1, payload);
        message.set_correlation_id(format!("test-{}", id));
        message
    }

    fn calculate_benchmark_result(
        &self,
        operation_name: &str,
        iterations: u32,
        times: Vec<Duration>,
        total_time: Duration,
    ) -> BenchmarkResult {
        let min_time = times.iter().min().copied().unwrap_or(Duration::ZERO);
        let max_time = times.iter().max().copied().unwrap_or(Duration::ZERO);
        let average_time = total_time / iterations;
        let operations_per_second = iterations as f64 / total_time.as_secs_f64();

        BenchmarkResult {
            operation_name: operation_name.to_string(),
            iterations,
            total_time,
            average_time,
            min_time,
            max_time,
            operations_per_second,
        }
    }
}

/// Performance optimization utilities
pub struct PerformanceOptimizer {
    metrics: Arc<MetricsCollector>,
}

impl PerformanceOptimizer {
    pub fn new(metrics: Arc<MetricsCollector>) -> Self {
        Self { metrics }
    }

    /// Analyze performance metrics and suggest optimizations
    pub fn analyze_performance(&self) -> Vec<String> {
        let metrics = self.metrics.get_metrics();
        let mut suggestions = Vec::new();

        // Check average processing time
        if metrics.average_processing_time > Duration::from_millis(100) {
            suggestions.push("High average processing time detected. Consider optimizing message parsing or I/O operations.".to_string());
        }

        // Check error rate
        if metrics.messages_processed > 0 {
            let error_rate = (metrics.messages_failed as f64 / metrics.messages_processed as f64) * 100.0;
            if error_rate > 5.0 {
                suggestions.push(format!("High error rate detected: {:.1}%. Review error handling and input validation.", error_rate));
            }
        }

        // Check worker utilization
        if metrics.worker_threads_active == 0 {
            suggestions.push("No worker threads active. Check worker thread management.".to_string());
        }

        // Check throughput
        let uptime_seconds = metrics.uptime.as_secs_f64();
        if uptime_seconds > 0.0 {
            let throughput = metrics.messages_processed as f64 / uptime_seconds;
            if throughput < 1.0 {
                suggestions.push("Low message throughput detected. Consider increasing worker threads or optimizing processing.".to_string());
            }
        }

        suggestions
    }

    /// Get performance recommendations based on current metrics
    pub fn get_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        let metrics = self.metrics.get_metrics();

        // Memory usage recommendations
        if metrics.total_bytes_processed > 1_000_000_000 { // 1GB
            recommendations.push("High memory usage detected. Consider implementing message batching or streaming.".to_string());
        }

        // Concurrency recommendations
        if metrics.worker_threads_active < 4 {
            recommendations.push("Consider increasing worker thread count for better concurrency.".to_string());
        }

        // Caching recommendations
        if metrics.version_requests > metrics.messages_processed / 2 {
            recommendations.push("High version request frequency. Consider implementing response caching.".to_string());
        }

        recommendations
    }
}
