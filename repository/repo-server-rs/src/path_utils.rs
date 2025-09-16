// Path sanitization utilities - equivalent to C++ createSanitizedPath method
use std::path::{Path, PathBuf};

/// Path sanitization result
#[derive(Debug, Clone)]
pub struct SanitizedPath {
    pub local_path: PathBuf,
    pub is_ambiguous: bool,
}

/// Path sanitization error
#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("Security violation: path {0} is outside base root")]
    SecurityViolation(String),
    #[error("Path ambiguity: both {0} and {1} exist")]
    PathAmbiguity(String, String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

/// Path sanitization utility - equivalent to C++ RequestWorker::createSanitizedPath
pub struct PathSanitizer {
    globus_collection_path: PathBuf,
}

impl PathSanitizer {
    pub fn new(globus_collection_path: impl Into<PathBuf>) -> Self {
        Self {
            globus_collection_path: globus_collection_path.into(),
        }
    }

    /// Sanitize a path - equivalent to C++ createSanitizedPath method
    pub fn sanitize_path(&self, path: &str) -> Result<SanitizedPath, PathError> {
        // Remove trailing slashes
        let mut sanitized_request_path = path.to_string();
        while sanitized_request_path.ends_with('/') && sanitized_request_path.len() > 1 {
            sanitized_request_path.pop();
        }

        if sanitized_request_path.is_empty() {
            return Err(PathError::InvalidPath("Empty path".to_string()));
        }

        // Check if path starts with globus_collection_path
        let globus_path_str = self.globus_collection_path.to_string_lossy();
        if sanitized_request_path.starts_with(&*globus_path_str) {
            // Path already contains the globus collection path
            self.handle_ambiguous_path(&sanitized_request_path, &globus_path_str)
        } else {
            // Path is relative - prepend globus collection path
            self.create_relative_path(&sanitized_request_path)
        }
    }

    /// Check if two strings have equal prefixes up to the specified length
    fn prefixes_equal(&self, str1: &str, str2: &str, length: usize) -> bool {
        if str1.len() < length || str2.len() < length {
            return false;
        }
        str1.chars().take(length).eq(str2.chars().take(length))
    }

    /// Handle potentially ambiguous paths - equivalent to C++ ambiguity detection
    fn handle_ambiguous_path(&self, sanitized_path: &str, globus_path: &str) -> Result<SanitizedPath, PathError> {
        // If the path already starts with the globus collection path, use it directly
        if sanitized_path.starts_with(&*globus_path) {
            let final_path = sanitized_path.to_string();
            return Ok(SanitizedPath {
                local_path: PathBuf::from(final_path),
                is_ambiguous: false,
            });
        }

        let local_path_1 = if sanitized_path.starts_with('/') {
            format!("{}{}", globus_path, sanitized_path)
        } else {
            if globus_path.ends_with('/') {
                format!("{}{}", globus_path, sanitized_path)
            } else {
                format!("{}/{}", globus_path, sanitized_path)
            }
        };

        let local_path_2 = if sanitized_path.starts_with('/') {
            sanitized_path.to_string()
        } else {
            format!("/{}", sanitized_path)
        };

        let path1 = Path::new(&local_path_1);
        let path2 = Path::new(&local_path_2);

        // Check if both paths exist and are different
        let exists1 = path1.exists();
        let exists2 = path2.exists();

        if exists1 && exists2 && local_path_1 != local_path_2 {
            return Err(PathError::PathAmbiguity(local_path_1, local_path_2));
        }

        // Use the shorter path (path2) if both exist and are the same
        // Otherwise use the appropriate path
        let final_path = if exists1 && exists2 {
            // Both exist and are the same - use the shorter one
            if local_path_1.len() <= local_path_2.len() {
                local_path_1.clone()
            } else {
                local_path_2.clone()
            }
        } else if exists1 {
            local_path_1.clone()
        } else if exists2 {
            local_path_2.clone()
        } else {
            // Neither exists - use the shorter path
            if local_path_1.len() <= local_path_2.len() {
                local_path_1.clone()
            } else {
                local_path_2.clone()
            }
        };

        Ok(SanitizedPath {
            local_path: PathBuf::from(final_path),
            is_ambiguous: exists1 && exists2 && local_path_1 != local_path_2,
        })
    }

    /// Create a relative path by prepending globus collection path
    fn create_relative_path(&self, sanitized_path: &str) -> Result<SanitizedPath, PathError> {
        let local_path = if sanitized_path.starts_with('/') {
            format!("{}{}", self.globus_collection_path.to_string_lossy(), sanitized_path)
        } else {
            let globus_str = self.globus_collection_path.to_string_lossy();
            if globus_str.ends_with('/') {
                format!("{}{}", globus_str, sanitized_path)
            } else {
                format!("{}/{}", globus_str, sanitized_path)
            }
        };

        Ok(SanitizedPath {
            local_path: PathBuf::from(local_path),
            is_ambiguous: false,
        })
    }

    /// Validate that a path is within the base root (security check)
    pub fn validate_path_security(&self, path: &Path) -> Result<(), PathError> {
        // Get the base path as a string for comparison
        let base_path_str = self.globus_collection_path.to_string_lossy();
        
        // Convert the target path to string for comparison
        let target_path_str = path.to_string_lossy();
        
        // Check if the target path starts with the base path
        if !target_path_str.starts_with(&*base_path_str) {
            return Err(PathError::SecurityViolation(format!(
                "Path '{}' is not within base path '{}'", 
                target_path_str, base_path_str
            )));
        }

        // Additional check: ensure the path doesn't contain ".." or other dangerous patterns
        if target_path_str.contains("..") || target_path_str.contains("//") {
            return Err(PathError::SecurityViolation(format!(
                "Path '{}' contains dangerous patterns", 
                target_path_str
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_path_sanitization() {
        let temp_dir = std::env::temp_dir().join("test_path_sanitization");
        let _ = fs::create_dir_all(&temp_dir);
        
        let sanitizer = PathSanitizer::new(&temp_dir);
        
        // Test relative path
        let result = sanitizer.sanitize_path("test/file.txt").unwrap();
        assert_eq!(result.local_path, temp_dir.join("test/file.txt"));
        assert!(!result.is_ambiguous);
        
        // Test absolute path
        let result = sanitizer.sanitize_path("/test/file.txt").unwrap();
        assert_eq!(result.local_path, temp_dir.join("test/file.txt"));
        assert!(!result.is_ambiguous);
    }

    #[test]
    fn test_security_validation() {
        let temp_dir = std::env::temp_dir().join("test_security");
        let _ = fs::create_dir_all(&temp_dir);
        
        let sanitizer = PathSanitizer::new(&temp_dir);
        
        // Valid path
        let valid_path = temp_dir.join("valid/file.txt");
        assert!(sanitizer.validate_path_security(&valid_path).is_ok());
        
        // Invalid path (path traversal)
        let invalid_path = temp_dir.join("../other/file.txt");
        assert!(sanitizer.validate_path_security(&invalid_path).is_err());
    }
}
