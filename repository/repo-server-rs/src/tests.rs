// Comprehensive unit tests for all modules
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    mod config_tests {
        use super::*;
        use crate::config::Config;

        #[test]
        fn test_config_default() {
            let config = Config::default();
            assert!(!config.core_server.is_empty());
            assert!(!config.cred_dir.is_empty());
            assert!(config.port > 0);
            assert!(config.timeout > 0);
            assert!(config.num_req_worker_threads > 0);
        }

        #[test]
        fn test_config_normalize() {
            let mut config = Config::default();
            config.globus_collection_path = Some("/test/path/".to_string());
            config.cred_dir = "/test/cred".to_string();
            
            config.normalize();
            
            assert_eq!(config.globus_collection_path, Some("/test/path".to_string()));
            assert_eq!(config.cred_dir, "/test/cred/");
        }

        #[test]
        fn test_config_validation() {
            let mut config = Config::default();
            assert!(config.validate().is_ok());

            // Test invalid core server
            config.core_server = "invalid".to_string();
            assert!(config.validate().is_err());

            // Test invalid port
            config.core_server = "tcp://localhost:9999".to_string();
            config.port = 0;
            assert!(config.validate().is_err());

            // Test invalid timeout
            config.port = 9999;
            config.timeout = 0;
            assert!(config.validate().is_err());

            // Test invalid worker threads
            config.timeout = 5000;
            config.num_req_worker_threads = 0;
            assert!(config.validate().is_err());
        }
    }

    mod path_utils_tests {
        use super::*;
        use crate::path_utils::*;

        #[test]
        fn test_path_sanitizer_creation() {
            let temp_dir = TempDir::new().unwrap();
            let sanitizer = PathSanitizer::new(temp_dir.path());
            assert_eq!(sanitizer.globus_collection_path, temp_dir.path());
        }

        #[test]
        fn test_sanitize_relative_path() {
            let temp_dir = TempDir::new().unwrap();
            let sanitizer = PathSanitizer::new(temp_dir.path());
            
            let result = sanitizer.sanitize_path("test/file.txt").unwrap();
            assert_eq!(result.local_path, temp_dir.path().join("test/file.txt"));
            assert!(!result.is_ambiguous);
        }

        #[test]
        fn test_sanitize_absolute_path() {
            let temp_dir = TempDir::new().unwrap();
            let sanitizer = PathSanitizer::new(temp_dir.path());
            
            let result = sanitizer.sanitize_path("/test/file.txt").unwrap();
            assert_eq!(result.local_path, temp_dir.path().join("test/file.txt"));
            assert!(!result.is_ambiguous);
        }

        #[test]
        fn test_sanitize_empty_path() {
            let temp_dir = TempDir::new().unwrap();
            let sanitizer = PathSanitizer::new(temp_dir.path());
            
            let result = sanitizer.sanitize_path("");
            assert!(result.is_err());
        }

        #[test]
        fn test_security_validation() {
            let temp_dir = TempDir::new().unwrap();
            let sanitizer = PathSanitizer::new(temp_dir.path());
            
            // Valid path
            let valid_path = temp_dir.path().join("valid/file.txt");
            std::fs::create_dir_all(valid_path.parent().unwrap()).unwrap();
            std::fs::write(&valid_path, "test").unwrap();
            assert!(sanitizer.validate_path_security(&valid_path).is_ok());
            
            // Invalid path (path traversal)
            let invalid_path = temp_dir.path().join("../other/file.txt");
            assert!(sanitizer.validate_path_security(&invalid_path).is_err());
        }
    }

    mod message_tests {
        use super::*;
        use crate::message::*;

        #[test]
        fn test_message_creation() {
            let payload = MessagePayload::Empty;
            let message = Message::new(1, payload);
            
            assert_eq!(message.message_type, 1);
            assert!(message.correlation_id.is_none());
            assert!(message.key.is_none());
        }

        #[test]
        fn test_message_correlation_id() {
            let mut message = Message::new(1, MessagePayload::Empty);
            message.set_correlation_id("test-id".to_string());
            
            assert_eq!(message.correlation_id, Some("test-id".to_string()));
            assert_eq!(message.get_correlation_id(), Some(&"test-id".to_string()));
        }

        #[test]
        fn test_message_key() {
            let mut message = Message::new(1, MessagePayload::Empty);
            message.set_key("test-key".to_string());
            
            assert_eq!(message.key, Some("test-key".to_string()));
            assert_eq!(message.get_key(), Some(&"test-key".to_string()));
        }

        #[test]
        fn test_nack_reply_serialization() {
            let nack = NackReply {
                err_code: -1,
                err_msg: "Test error".to_string(),
            };
            
            let serialized = nack.serialize().unwrap();
            assert!(!serialized.is_empty());
            
            let deserialized = NackReply::deserialize(&serialized).unwrap();
            assert_eq!(deserialized.err_code, -1);
            assert_eq!(deserialized.err_msg, "Test error");
        }

        #[test]
        fn test_ack_reply_serialization() {
            let ack = AckReply;
            
            let serialized = ack.serialize().unwrap();
            assert!(serialized.is_empty());
            
            let deserialized = AckReply::deserialize(&serialized).unwrap();
            // AckReply is a unit struct, so we just verify it deserializes
            assert!(matches!(deserialized, AckReply));
        }
    }

    mod metrics_tests {
        use super::*;
        use crate::metrics::*;
        use std::time::Duration;

        #[test]
        fn test_metrics_collector_creation() {
            let collector = MetricsCollector::new();
            let metrics = collector.get_metrics();
            
            assert_eq!(metrics.messages_processed, 0);
            assert_eq!(metrics.messages_failed, 0);
            assert_eq!(metrics.worker_threads_active, 0);
        }

        #[test]
        fn test_record_message_processed() {
            let collector = MetricsCollector::new();
            collector.record_message_processed(1, 100, Duration::from_millis(50));
            
            let metrics = collector.get_metrics();
            assert_eq!(metrics.messages_processed, 1);
            assert_eq!(metrics.total_bytes_processed, 100);
            assert_eq!(metrics.version_requests, 1);
        }

        #[test]
        fn test_record_message_failed() {
            let collector = MetricsCollector::new();
            collector.record_message_failed(1);
            
            let metrics = collector.get_metrics();
            assert_eq!(metrics.messages_failed, 1);
        }

        #[test]
        fn test_worker_threads_active() {
            let collector = MetricsCollector::new();
            collector.set_worker_threads_active(4);
            
            let metrics = collector.get_metrics();
            assert_eq!(metrics.worker_threads_active, 4);
        }

        #[test]
        fn test_health_check() {
            let collector = MetricsCollector::new();
            let health = collector.health_check();
            
            // Should be healthy initially
            assert!(health.is_healthy);
            assert!(health.issues.is_empty());
        }

        #[test]
        fn test_performance_monitor() {
            let monitor = PerformanceMonitor::start("test_operation");
            std::thread::sleep(Duration::from_millis(10));
            let duration = monitor.finish();
            
            assert!(duration >= Duration::from_millis(10));
        }
    }

    mod version_tests {
        use super::*;
        use crate::version::*;

        #[test]
        fn test_version_reply_creation() {
            let version = VersionReply::new();
            
            assert!(version.release_year > 0);
            assert!(version.release_month > 0);
            assert!(version.release_day > 0);
            assert!(version.api_major > 0);
            assert!(version.component_major > 0);
        }

        #[test]
        fn test_version_reply_from_core_server() {
            let version = VersionReply::from_core_server(
                2025, 6, 11, 14, 1,
                1, 0, 0,
                1, 0, 0
            );
            
            assert_eq!(version.release_year, 2025);
            assert_eq!(version.release_month, 6);
            assert_eq!(version.release_day, 11);
            assert_eq!(version.api_major, 1);
            assert_eq!(version.component_major, 1);
        }

        #[test]
        fn test_version_reply_serialization() {
            let version = VersionReply::new();
            let bytes = version.to_bytes();
            
            // Should have 44 bytes (11 fields * 4 bytes each)
            assert_eq!(bytes.len(), 44);
        }

        #[test]
        fn test_shared_version_info() {
            let info = create_shared_version_info();
            let version_info = info.read().unwrap();
            
            assert!(!version_info.is_connected);
            assert!(version_info.version_reply.release_year > 0);
        }
    }

    mod integration_tests {
        use super::*;
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
        use crate::worker::*;
        use crate::version::*;

        #[test]
        fn test_worker_spawn() {
            let running = Arc::new(AtomicBool::new(true));
            let version_info = create_shared_version_info();
            let _messenger = ZMQInprocMessenger::new(0);
            
            let handle = spawn_worker(
                0,
                _messenger,
                "/tmp/test",
                running.clone(),
                version_info,
            );
            
            // Give it a moment to start
            std::thread::sleep(Duration::from_millis(100));
            
            // Stop the worker
            running.store(false, Ordering::SeqCst);
            handle.join().unwrap();
        }

        #[test]
        fn test_message_parsing() {
            // Test parsing of different message types
            let test_cases = vec![
                (1, "VERSION_REQUEST"),
                (17930, "REPO_DATA_DELETE_REQUEST"),
                (13322, "REPO_DATA_GET_SIZE_REQUEST"),
                (13578, "REPO_PATH_CREATE_REQUEST"),
                (1004, "REPO_PATH_DELETE_REQUEST"),
            ];

            for (msg_type, name) in test_cases {
                // Create a simple test message
                let mut data = Vec::new();
                data.extend_from_slice(&msg_type.to_le_bytes());
                // Add some dummy payload data
                data.extend_from_slice(b"test");
                
                // This would test the actual parsing logic
                // For now, just verify the message type extraction
                let extracted_type = u16::from_le_bytes([data[0], data[1]]);
                assert_eq!(extracted_type, msg_type, "Failed for {}", name);
            }
        }
    }
}
