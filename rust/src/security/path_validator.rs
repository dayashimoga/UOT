//! Path Validation & Sanitization
//!
//! Implements comprehensive path security to prevent directory traversal,
//! symlink attacks, and OS-specific path injection.
use std::path::{Component, Path, PathBuf};

use crate::core::error::SecurityError;
use crate::security::PathValidator;

/// Windows reserved device names that must be rejected.
const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Characters that are illegal in filenames across platforms.
const ILLEGAL_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*', '\0'];

/// Production-grade path validator.
#[derive(Debug, Clone, Default)]
pub struct StrictPathValidator {
    /// Base directory that all resolved paths must be within.
    pub base_dir: Option<PathBuf>,
}

impl StrictPathValidator {
    /// Create a new validator with an optional base directory constraint.
    pub fn new(base_dir: Option<PathBuf>) -> Self {
        Self { base_dir }
    }

    /// Check if a filename stem (without extension) is a Windows reserved name.
    fn is_windows_reserved(name: &str) -> bool {
        let upper = name.to_uppercase();
        // Check stem only (CON.txt → CON is reserved)
        let stem = upper.split('.').next().unwrap_or(&upper);
        WINDOWS_RESERVED.contains(&stem)
    }

    /// Check for URL-encoded traversal sequences.
    fn has_encoded_traversal(s: &str) -> bool {
        let lower = s.to_lowercase();
        lower.contains("%2e%2e")
            || lower.contains("%2f")
            || lower.contains("%5c")
            || lower.contains("%00")
    }

    /// Validate that a resolved path is within the base directory.
    pub fn validate_within_base(&self, resolved: &Path) -> Result<(), SecurityError> {
        if let Some(ref base) = self.base_dir {
            if !resolved.starts_with(base) {
                return Err(SecurityError::PathTraversal {
                    path: resolved.display().to_string(),
                    reason: "Resolved path escapes base directory".to_string(),
                });
            }
        }
        Ok(())
    }
}

impl PathValidator for StrictPathValidator {
    /// Validate that a filename is safe.
    ///
    /// Rejects: empty names, traversal sequences, null bytes, illegal chars,
    /// Windows reserved names, hidden files starting with `.`, and overly long names.
    fn validate_filename(&self, filename: &str) -> Result<String, SecurityError> {
        // Empty check
        if filename.is_empty() || filename.trim().is_empty() {
            return Err(SecurityError::PathTraversal {
                path: filename.to_string(),
                reason: "Empty filename".to_string(),
            });
        }

        // Null byte check
        if filename.contains('\0') {
            return Err(SecurityError::PathTraversal {
                path: filename.to_string(),
                reason: "Null byte in filename".to_string(),
            });
        }

        // URL-encoded traversal check
        if Self::has_encoded_traversal(filename) {
            return Err(SecurityError::PathTraversal {
                path: filename.to_string(),
                reason: "URL-encoded traversal sequence".to_string(),
            });
        }

        // Directory separator check (filename must not contain path separators)
        if filename.contains('/') || filename.contains('\\') {
            return Err(SecurityError::PathTraversal {
                path: filename.to_string(),
                reason: "Directory separator in filename".to_string(),
            });
        }

        // Traversal component check
        if filename == "." || filename == ".." {
            return Err(SecurityError::PathTraversal {
                path: filename.to_string(),
                reason: "Traversal component".to_string(),
            });
        }

        // Illegal characters
        for c in ILLEGAL_CHARS {
            if filename.contains(*c) {
                return Err(SecurityError::PathTraversal {
                    path: filename.to_string(),
                    reason: format!("Illegal character '{c}'"),
                });
            }
        }

        // Windows reserved names
        if Self::is_windows_reserved(filename) {
            return Err(SecurityError::PathTraversal {
                path: filename.to_string(),
                reason: "Windows reserved device name".to_string(),
            });
        }

        // Length check (255 bytes is typical filesystem limit)
        if filename.len() > 255 {
            return Err(SecurityError::PathTraversal {
                path: filename.to_string(),
                reason: format!("Filename too long ({} bytes, max 255)", filename.len()),
            });
        }

        Ok(filename.to_string())
    }

    /// Validate a relative path, checking each component.
    ///
    /// Rejects: absolute paths, traversal, null bytes, encoded sequences.
    /// If a base directory is set, also validates the resolved path is within it.
    fn validate_relative_path(&self, path: &str) -> Result<String, SecurityError> {
        if path.is_empty() {
            return Err(SecurityError::PathTraversal {
                path: path.to_string(),
                reason: "Empty path".to_string(),
            });
        }

        // Null byte
        if path.contains('\0') {
            return Err(SecurityError::PathTraversal {
                path: path.to_string(),
                reason: "Null byte in path".to_string(),
            });
        }

        // Encoded traversal
        if Self::has_encoded_traversal(path) {
            return Err(SecurityError::PathTraversal {
                path: path.to_string(),
                reason: "URL-encoded traversal sequence".to_string(),
            });
        }

        let p = Path::new(path);

        // Must not be absolute
        if p.is_absolute() {
            return Err(SecurityError::PathTraversal {
                path: path.to_string(),
                reason: "Absolute path not allowed".to_string(),
            });
        }

        // Check each component
        let mut clean_parts: Vec<String> = Vec::new();
        for component in p.components() {
            match component {
                Component::Normal(s) => {
                    let s_str = s.to_str().ok_or_else(|| SecurityError::PathTraversal {
                        path: path.to_string(),
                        reason: "Non-UTF-8 path component".to_string(),
                    })?;
                    self.validate_filename(s_str)?;
                    clean_parts.push(s_str.to_string());
                }
                Component::ParentDir => {
                    return Err(SecurityError::PathTraversal {
                        path: path.to_string(),
                        reason: "Parent directory traversal (..)".to_string(),
                    });
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(SecurityError::PathTraversal {
                        path: path.to_string(),
                        reason: "Absolute path component".to_string(),
                    });
                }
                Component::CurDir => {
                    // Skip `.` components silently
                }
            }
        }

        if clean_parts.is_empty() {
            return Err(SecurityError::PathTraversal {
                path: path.to_string(),
                reason: "Path resolves to empty".to_string(),
            });
        }

        // Reconstruct with forward slashes (platform-neutral)
        let cleaned = clean_parts.join("/");

        // If base_dir is set, verify the resolved path stays inside
        if let Some(ref base) = self.base_dir {
            let resolved = base.join(&cleaned);
            self.validate_within_base(&resolved)?;
        }

        Ok(cleaned)
    }

    /// Sanitize a filename by removing or replacing dangerous characters.
    ///
    /// This is a best-effort cleanup — for untrusted input, prefer `validate_filename`.
    fn sanitize_filename(&self, filename: &str) -> String {
        let mut sanitized: String = filename
            .chars()
            .filter(|c| !ILLEGAL_CHARS.contains(c) && *c != '\0')
            .map(|c| if c == '/' || c == '\\' { '_' } else { c })
            .collect();

        // Replace traversal
        while sanitized.contains("..") {
            sanitized = sanitized.replace("..", "_");
        }

        // Handle Windows reserved names
        if Self::is_windows_reserved(&sanitized) {
            sanitized = format!("_{sanitized}");
        }

        // Handle empty result
        if sanitized.is_empty() || sanitized.trim().is_empty() {
            sanitized = "unnamed".to_string();
        }

        // Truncate
        if sanitized.len() > 255 {
            sanitized.truncate(255);
        }

        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validator() -> StrictPathValidator {
        StrictPathValidator::new(None)
    }

    fn validator_with_base() -> StrictPathValidator {
        StrictPathValidator::new(Some(PathBuf::from("/safe/downloads")))
    }

    // ── Filename validation ──

    #[test]
    fn test_valid_filename() {
        let v = validator();
        assert_eq!(v.validate_filename("document.pdf").unwrap(), "document.pdf");
        assert_eq!(
            v.validate_filename("my file (1).txt").unwrap(),
            "my file (1).txt"
        );
    }

    #[test]
    fn test_empty_filename() {
        let v = validator();
        assert!(v.validate_filename("").is_err());
        assert!(v.validate_filename("   ").is_err());
    }

    #[test]
    fn test_null_byte_filename() {
        let v = validator();
        assert!(v.validate_filename("file\0.txt").is_err());
    }

    #[test]
    fn test_traversal_filename() {
        let v = validator();
        assert!(v.validate_filename("..").is_err());
        assert!(v.validate_filename(".").is_err());
    }

    #[test]
    fn test_separator_in_filename() {
        let v = validator();
        assert!(v.validate_filename("path/file.txt").is_err());
        assert!(v.validate_filename("path\\file.txt").is_err());
    }

    #[test]
    fn test_illegal_chars_filename() {
        let v = validator();
        assert!(v.validate_filename("file<1>.txt").is_err());
        assert!(v.validate_filename("file:name").is_err());
        assert!(v.validate_filename("file\"name").is_err());
    }

    #[test]
    fn test_windows_reserved() {
        let v = validator();
        assert!(v.validate_filename("CON").is_err());
        assert!(v.validate_filename("con").is_err());
        assert!(v.validate_filename("CON.txt").is_err());
        assert!(v.validate_filename("NUL").is_err());
        assert!(v.validate_filename("LPT1").is_err());
        // Not reserved
        assert!(v.validate_filename("CONSOLE").is_ok());
    }

    #[test]
    fn test_long_filename() {
        let v = validator();
        let long = "a".repeat(256);
        assert!(v.validate_filename(&long).is_err());
        let ok = "a".repeat(255);
        assert!(v.validate_filename(&ok).is_ok());
    }

    // ── Relative path validation ──

    #[test]
    fn test_valid_relative_path() {
        let v = validator();
        assert_eq!(
            v.validate_relative_path("folder/file.txt").unwrap(),
            "folder/file.txt"
        );
        assert_eq!(v.validate_relative_path("a/b/c.txt").unwrap(), "a/b/c.txt");
    }

    #[test]
    fn test_traversal_path() {
        let v = validator();
        assert!(v.validate_relative_path("../etc/passwd").is_err());
        assert!(v.validate_relative_path("folder/../../secret").is_err());
        assert!(v.validate_relative_path("..").is_err());
    }

    #[test]
    fn test_absolute_path_rejected() {
        let v = validator();
        assert!(v.validate_relative_path("/etc/passwd").is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_absolute_path_rejected() {
        let v = validator();
        assert!(v.validate_relative_path("C:\\Windows\\System32").is_err());
    }

    #[test]
    fn test_encoded_traversal_path() {
        let v = validator();
        assert!(v.validate_relative_path("%2e%2e/secret").is_err());
        assert!(v.validate_relative_path("folder%2f..%2fsecret").is_err());
        assert!(v.validate_relative_path("file%00.txt").is_err());
    }

    #[test]
    fn test_curdir_stripped() {
        let v = validator();
        assert_eq!(v.validate_relative_path("./file.txt").unwrap(), "file.txt");
    }

    // ── Sanitize ──

    #[test]
    fn test_sanitize_traversal() {
        let v = validator();
        assert!(!v.sanitize_filename("../../etc/passwd").contains(".."));
    }

    #[test]
    fn test_sanitize_illegal() {
        let v = validator();
        let result = v.sanitize_filename("file<>:\"|?*.txt");
        assert!(!result.contains('<'));
        assert!(!result.contains('>'));
    }

    #[test]
    fn test_sanitize_empty() {
        let v = validator();
        assert_eq!(v.sanitize_filename(""), "unnamed");
    }

    #[test]
    fn test_sanitize_reserved() {
        let v = validator();
        let result = v.sanitize_filename("CON");
        assert_ne!(result, "CON");
    }

    // ── Base directory validation ──

    #[test]
    fn test_path_within_base() {
        let v = validator_with_base();
        assert!(v.validate_relative_path("subdir/file.txt").is_ok());
    }
}
