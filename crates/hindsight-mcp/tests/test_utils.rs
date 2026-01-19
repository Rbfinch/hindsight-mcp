// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Test utilities for hindsight-mcp integration tests
//!
//! This module provides utilities for:
//! - Temporary directory management
//! - Git repository scaffolding for tests
//! - JSON comparison helpers
//! - Environment isolation

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// Temporary Directory Management
// ============================================================================

/// Counter for generating unique test directory names
static TEST_DIR_COUNTER: AtomicU32 = AtomicU32::new(0);

/// A temporary directory that is automatically cleaned up when dropped
///
/// This provides a unique, isolated directory for each test to avoid
/// interference between concurrent tests.
pub struct TempTestDir {
    path: PathBuf,
    cleanup: bool,
}

impl TempTestDir {
    /// Create a new temporary test directory
    ///
    /// The directory is created under the system temp directory with a
    /// unique name based on the test name and a counter.
    pub fn new(test_name: &str) -> Self {
        let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir_name = format!(
            "hindsight-test-{}-{}-{}",
            test_name,
            std::process::id(),
            counter
        );
        let path = std::env::temp_dir().join(dir_name);

        fs::create_dir_all(&path).expect("Failed to create temp test directory");

        Self {
            path,
            cleanup: true,
        }
    }

    /// Create a temp directory and don't clean it up (for debugging)
    #[allow(dead_code)]
    pub fn new_persistent(test_name: &str) -> Self {
        let mut temp = Self::new(test_name);
        temp.cleanup = false;
        eprintln!("Persistent temp dir: {}", temp.path.display());
        temp
    }

    /// Get the path to the temporary directory
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Create a subdirectory within the temp directory
    #[allow(dead_code)]
    pub fn create_subdir(&self, name: &str) -> PathBuf {
        let subdir = self.path.join(name);
        fs::create_dir_all(&subdir).expect("Failed to create subdirectory");
        subdir
    }

    /// Create a file within the temp directory with the given content
    pub fn create_file(&self, relative_path: &str, content: &str) -> PathBuf {
        let file_path = self.path.join(relative_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directories");
        }
        fs::write(&file_path, content).expect("Failed to write file");
        file_path
    }

    /// Read a file from the temp directory
    pub fn read_file(&self, relative_path: &str) -> String {
        let file_path = self.path.join(relative_path);
        fs::read_to_string(&file_path).expect("Failed to read file")
    }

    /// Check if a file exists in the temp directory
    #[allow(dead_code)]
    pub fn file_exists(&self, relative_path: &str) -> bool {
        self.path.join(relative_path).exists()
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        if self.cleanup && self.path.exists() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

// ============================================================================
// Git Repository Scaffolding
// ============================================================================

/// A temporary git repository for testing
///
/// This creates a real git repository with configurable commits,
/// useful for testing git ingestion functionality.
pub struct TestGitRepo {
    temp_dir: TempTestDir,
    initialized: bool,
}

impl TestGitRepo {
    /// Create a new test git repository
    pub fn new(test_name: &str) -> Self {
        let temp_dir = TempTestDir::new(test_name);
        Self {
            temp_dir,
            initialized: false,
        }
    }

    /// Initialize the git repository
    pub fn init(&mut self) -> &mut Self {
        if !self.initialized {
            run_git(&self.temp_dir.path, &["init"]);
            run_git(
                &self.temp_dir.path,
                &["config", "user.email", "test@example.com"],
            );
            run_git(&self.temp_dir.path, &["config", "user.name", "Test Author"]);
            self.initialized = true;
        }
        self
    }

    /// Get the path to the repository
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Create a file and stage it
    pub fn create_file(&self, relative_path: &str, content: &str) -> &Self {
        self.temp_dir.create_file(relative_path, content);
        run_git(self.temp_dir.path(), &["add", relative_path]);
        self
    }

    /// Create a commit with the given message
    pub fn commit(&self, message: &str) -> String {
        run_git(
            self.temp_dir.path(),
            &["commit", "--allow-empty", "-m", message],
        );
        self.get_head_sha()
    }

    /// Create a file and commit it in one step
    pub fn create_and_commit(&self, relative_path: &str, content: &str, message: &str) -> String {
        self.create_file(relative_path, content);
        self.commit(message)
    }

    /// Get the SHA of HEAD
    pub fn get_head_sha(&self) -> String {
        let output = Command::new("git")
            .current_dir(self.temp_dir.path())
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("Failed to get HEAD SHA");

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Get the number of commits in the repository
    pub fn commit_count(&self) -> usize {
        let output = Command::new("git")
            .current_dir(self.temp_dir.path())
            .args(["rev-list", "--count", "HEAD"])
            .output()
            .expect("Failed to count commits");

        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .unwrap_or(0)
    }

    /// Create multiple commits for testing
    pub fn create_commits(&self, count: usize) -> Vec<String> {
        (0..count)
            .map(|i| {
                self.create_and_commit(
                    &format!("file_{}.txt", i),
                    &format!("Content {}", i),
                    &format!("Commit {}", i),
                )
            })
            .collect()
    }
}

/// Run a git command in the given directory
fn run_git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to run git command");

    if !output.status.success() {
        panic!(
            "Git command failed: git {}\nstderr: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

// ============================================================================
// JSON Comparison Helpers
// ============================================================================

/// Compare two JSON values, ignoring field order
pub fn json_eq(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a, b) {
        (serde_json::Value::Object(a_map), serde_json::Value::Object(b_map)) => {
            if a_map.len() != b_map.len() {
                return false;
            }
            a_map
                .iter()
                .all(|(key, val)| b_map.get(key).is_some_and(|b_val| json_eq(val, b_val)))
        }
        (serde_json::Value::Array(a_arr), serde_json::Value::Array(b_arr)) => {
            if a_arr.len() != b_arr.len() {
                return false;
            }
            a_arr.iter().zip(b_arr.iter()).all(|(a, b)| json_eq(a, b))
        }
        _ => a == b,
    }
}

/// Assert that two JSON values are equal, with a nice diff on failure
#[allow(dead_code)]
pub fn assert_json_eq(actual: &serde_json::Value, expected: &serde_json::Value) {
    if !json_eq(actual, expected) {
        let actual_pretty = serde_json::to_string_pretty(actual).unwrap();
        let expected_pretty = serde_json::to_string_pretty(expected).unwrap();
        panic!(
            "JSON values do not match:\n\nActual:\n{}\n\nExpected:\n{}",
            actual_pretty, expected_pretty
        );
    }
}

/// Check if a JSON value contains a field with a specific value
pub fn json_contains_field(
    json: &serde_json::Value,
    field: &str,
    value: &serde_json::Value,
) -> bool {
    match json {
        serde_json::Value::Object(map) => map.get(field).is_some_and(|v| json_eq(v, value)),
        _ => false,
    }
}

/// Check if a JSON array contains an object with a specific field value
pub fn json_array_contains(
    array: &serde_json::Value,
    field: &str,
    value: &serde_json::Value,
) -> bool {
    match array {
        serde_json::Value::Array(arr) => arr
            .iter()
            .any(|item| json_contains_field(item, field, value)),
        _ => false,
    }
}

// ============================================================================
// Environment Isolation
// ============================================================================

/// Temporarily set an environment variable for a test
///
/// The original value is restored when the guard is dropped.
pub struct EnvGuard {
    key: String,
    original: Option<String>,
}

impl EnvGuard {
    /// Set an environment variable, returning a guard that restores it on drop
    pub fn set(key: &str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        // SAFETY: We're in test code and control the environment variable access
        unsafe { std::env::set_var(key, value) };
        Self {
            key: key.to_string(),
            original,
        }
    }

    /// Remove an environment variable, returning a guard that restores it on drop
    #[allow(dead_code)]
    pub fn remove(key: &str) -> Self {
        let original = std::env::var(key).ok();
        // SAFETY: We're in test code and control the environment variable access
        unsafe { std::env::remove_var(key) };
        Self {
            key: key.to_string(),
            original,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: We're in test code and control the environment variable access
        unsafe {
            match &self.original {
                Some(val) => std::env::set_var(&self.key, val),
                None => std::env::remove_var(&self.key),
            }
        }
    }
}

// ============================================================================
// Nextest JSON Fixtures
// ============================================================================

/// Generate a sample nextest JSON output for testing
///
/// This creates valid libtest-json format output that can be used
/// to test the ingest command.
pub fn sample_nextest_json(passed: usize, failed: usize, ignored: usize) -> String {
    let mut lines = Vec::new();

    // Suite started event
    lines.push(r#"{"type":"suite","event":"started","test_count":0}"#.to_string());

    // Generate passing tests
    for i in 0..passed {
        lines.push(format!(
            r#"{{"type":"test","event":"started","name":"test_passes_{}"}}"#,
            i
        ));
        lines.push(format!(
            r#"{{"type":"test","event":"ok","name":"test_passes_{}","exec_time":0.001}}"#,
            i
        ));
    }

    // Generate failing tests
    for i in 0..failed {
        lines.push(format!(
            r#"{{"type":"test","event":"started","name":"test_fails_{}"}}"#,
            i
        ));
        lines.push(format!(
            r#"{{"type":"test","event":"failed","name":"test_fails_{}","exec_time":0.001,"stdout":"assertion failed"}}"#,
            i
        ));
    }

    // Generate ignored tests
    for i in 0..ignored {
        lines.push(format!(
            r#"{{"type":"test","event":"ignored","name":"test_ignored_{}"}}"#,
            i
        ));
    }

    // Suite finished event
    lines.push(format!(
        r#"{{"type":"suite","event":"{}","passed":{},"failed":{},"ignored":{},"measured":0,"filtered_out":0,"exec_time":0.1}}"#,
        if failed > 0 { "failed" } else { "ok" },
        passed,
        failed,
        ignored
    ));

    lines.join("\n")
}

// ============================================================================
// Unit Tests for Utilities
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_utils_temp_dir_creation() {
        let temp = TempTestDir::new("test_creation");
        assert!(temp.path().exists());
        assert!(temp.path().is_dir());
    }

    #[test]
    fn test_utils_temp_dir_cleanup() {
        let path;
        {
            let temp = TempTestDir::new("test_cleanup");
            path = temp.path().to_path_buf();
            assert!(path.exists());
        }
        // Directory should be cleaned up after drop
        assert!(!path.exists());
    }

    #[test]
    fn test_utils_temp_dir_create_file() {
        let temp = TempTestDir::new("test_create_file");
        let file_path = temp.create_file("subdir/test.txt", "hello world");

        assert!(file_path.exists());
        assert_eq!(temp.read_file("subdir/test.txt"), "hello world");
    }

    #[test]
    fn test_utils_git_repo_init() {
        let mut repo = TestGitRepo::new("test_git_init");
        repo.init();

        assert!(repo.path().join(".git").exists());
    }

    #[test]
    fn test_utils_git_repo_commit() {
        let mut repo = TestGitRepo::new("test_git_commit");
        repo.init();

        let sha = repo.create_and_commit("file.txt", "content", "Initial commit");

        assert!(!sha.is_empty());
        assert_eq!(sha.len(), 40); // Full SHA
        assert_eq!(repo.commit_count(), 1);
    }

    #[test]
    fn test_utils_git_repo_multiple_commits() {
        let mut repo = TestGitRepo::new("test_git_multiple");
        repo.init();

        let shas = repo.create_commits(5);

        assert_eq!(shas.len(), 5);
        assert_eq!(repo.commit_count(), 5);
    }

    #[test]
    fn test_utils_json_eq_simple() {
        let a = json!({"name": "test", "value": 42});
        let b = json!({"value": 42, "name": "test"});

        assert!(json_eq(&a, &b));
    }

    #[test]
    fn test_utils_json_eq_nested() {
        let a = json!({"outer": {"inner": [1, 2, 3]}});
        let b = json!({"outer": {"inner": [1, 2, 3]}});

        assert!(json_eq(&a, &b));
    }

    #[test]
    fn test_utils_json_eq_not_equal() {
        let a = json!({"name": "test"});
        let b = json!({"name": "different"});

        assert!(!json_eq(&a, &b));
    }

    #[test]
    fn test_utils_json_array_contains() {
        let array = json!([
            {"id": 1, "name": "first"},
            {"id": 2, "name": "second"}
        ]);

        assert!(json_array_contains(&array, "name", &json!("second")));
        assert!(!json_array_contains(&array, "name", &json!("third")));
    }

    #[test]
    fn test_utils_env_guard_set() {
        let key = "HINDSIGHT_TEST_ENV_VAR";
        // SAFETY: We're in test code and control the environment variable access
        unsafe { std::env::remove_var(key) };

        {
            let _guard = EnvGuard::set(key, "test_value");
            assert_eq!(std::env::var(key).ok(), Some("test_value".to_string()));
        }

        // Should be removed after guard is dropped
        assert!(std::env::var(key).is_err());
    }

    #[test]
    fn test_utils_nextest_json_format() {
        let json = sample_nextest_json(2, 1, 1);

        // Should contain multiple lines
        let lines: Vec<&str> = json.lines().collect();
        assert!(lines.len() > 5);

        // Each line should be valid JSON
        for line in &lines {
            let _: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|_| panic!("Invalid JSON line: {}", line));
        }
    }
}
