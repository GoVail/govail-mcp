use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionLevel {
    ReadOnly = 1,
    ContainedWrite = 2,
    IsolatedExec = 3,
    SystemAdmin = 4,
}

pub struct PiiGuardrail;

impl PiiGuardrail {
    pub fn new() -> Self {
        Self
    }

    pub fn sanitize(&self, text: &str) -> String {
        let mut result = String::new();
        for word in text.split_whitespace() {
            if word.contains('@') && word.contains('.') {
                result.push_str("[REDACTED_EMAIL] ");
            } else if word.len() >= 10 && word.chars().all(|c| c.is_ascii_digit() || c == '-') {
                result.push_str("[REDACTED_PHONE] ");
            } else {
                result.push_str(word);
                result.push(' ');
            }
        }
        result.trim_end().to_string()
    }
}

pub struct SandboxGuardrail {
    max_allowed_level: PermissionLevel,
    workspace_root: String,
}

impl SandboxGuardrail {
    pub fn new(max_allowed_level: PermissionLevel, workspace_root: impl Into<String>) -> Self {
        Self {
            max_allowed_level,
            workspace_root: workspace_root.into(),
        }
    }

    pub fn validate_execution(
        &self,
        _tool_name: &str,
        required_level: PermissionLevel,
        target_path: Option<&str>,
        command: Option<&str>,
    ) -> Result<(), String> {
        // 1. Permission level check
        if required_level > self.max_allowed_level {
            return Err(format!(
                "Permission Denied: Required level {:?} exceeds max allowed level {:?}",
                required_level, self.max_allowed_level
            ));
        }

        // 2. Directory traversal and path restriction check
        if let Some(path) = target_path {
            if path.contains("..") {
                return Err(format!("Security Violation: Path traversal detected in '{}'", path));
            }
            if path.starts_with('/') && !path.starts_with(&self.workspace_root) {
                return Err(format!(
                    "Security Violation: Target path '{}' outside workspace root '{}'",
                    path, self.workspace_root
                ));
            }
        }

        // 3. Dangerous command check
        if let Some(cmd) = command {
            let dangerous_keywords = ["rm -rf /", "chmod 777", "curl | bash", "wget | sh", "> /dev/sda"];
            for keyword in &dangerous_keywords {
                if cmd.contains(keyword) {
                    return Err(format!("Security Violation: Dangerous command pattern detected: '{}'", keyword));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pii_guardrail() {
        let guard = PiiGuardrail::new();
        let text = "Contact test@example.com or 010-1234-5678 for details";
        let clean = guard.sanitize(text);
        assert!(clean.contains("[REDACTED_EMAIL]"));
        assert!(clean.contains("[REDACTED_PHONE]"));
    }

    #[test]
    fn test_sandbox_permission_denied() {
        let sandbox = SandboxGuardrail::new(PermissionLevel::ReadOnly, "/tmp/workspace");
        let res = sandbox.validate_execution("write_file", PermissionLevel::ContainedWrite, Some("/tmp/workspace/file.txt"), None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Permission Denied"));
    }

    #[test]
    fn test_sandbox_path_traversal_blocked() {
        let sandbox = SandboxGuardrail::new(PermissionLevel::ContainedWrite, "/tmp/workspace");
        let res = sandbox.validate_execution("write_file", PermissionLevel::ContainedWrite, Some("/tmp/workspace/../../etc/passwd"), None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Path traversal"));
    }

    #[test]
    fn test_sandbox_dangerous_command_blocked() {
        let sandbox = SandboxGuardrail::new(PermissionLevel::IsolatedExec, "/tmp/workspace");
        let res = sandbox.validate_execution("run_command", PermissionLevel::IsolatedExec, None, Some("rm -rf / --no-preserve-root"));
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Dangerous command pattern"));
    }

    #[test]
    fn test_sandbox_valid_execution() {
        let sandbox = SandboxGuardrail::new(PermissionLevel::ContainedWrite, "/tmp/workspace");
        let res = sandbox.validate_execution("write_file", PermissionLevel::ContainedWrite, Some("/tmp/workspace/src/lib.rs"), None);
        assert!(res.is_ok());
    }
}
