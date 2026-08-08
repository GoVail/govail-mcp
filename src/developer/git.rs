use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::service::DeliveryError;

#[derive(Debug, Clone)]
pub struct GitRepository {
    root: PathBuf,
    git_dir: PathBuf,
}

impl GitRepository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DeliveryError> {
        let requested = path.as_ref();
        let root_output = run_at(requested, ["rev-parse", "--show-toplevel"])?;
        let root = PathBuf::from(output_text(root_output)?).canonicalize()?;
        let git_dir_output = run_at(&root, ["rev-parse", "--git-dir"])?;
        let raw_git_dir = PathBuf::from(output_text(git_dir_output)?);
        let git_dir = if raw_git_dir.is_absolute() {
            raw_git_dir
        } else {
            root.join(raw_git_dir)
        }
        .canonicalize()?;
        Ok(Self { root, git_dir })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    pub fn output<I, S>(&self, args: I) -> Result<String, DeliveryError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        output_text(run_at(&self.root, args)?)
    }

    pub fn status<I, S>(&self, args: I) -> Result<(), DeliveryError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = run_at(&self.root, args)?;
        if output.status.success() {
            return Ok(());
        }
        Err(command_error(output))
    }

    pub fn head(&self) -> Result<String, DeliveryError> {
        self.output(["rev-parse", "HEAD"])
    }

    pub fn parent(&self) -> Result<Option<String>, DeliveryError> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD^"])
            .current_dir(&self.root)
            .output()?;
        if output.status.success() {
            Ok(Some(String::from_utf8(output.stdout)?.trim().to_owned()))
        } else {
            Ok(None)
        }
    }

    pub fn current_branch(&self) -> Result<Option<String>, DeliveryError> {
        let branch = self.output(["branch", "--show-current"])?;
        Ok((!branch.is_empty()).then_some(branch))
    }

    pub fn changed_paths(&self) -> Result<Vec<String>, DeliveryError> {
        let output = run_at(
            &self.root,
            [
                "status",
                "--short",
                "--no-renames",
                "--untracked-files=all",
                "-z",
            ],
        )?;
        let fields = nul_fields(output.stdout)?;
        Ok(fields
            .into_iter()
            .filter_map(|field| field.get(3..).map(str::to_owned))
            .collect())
    }

    pub fn remote_url(&self, remote: &str) -> Result<String, DeliveryError> {
        self.output(["remote", "get-url", remote])
    }

    pub fn remote_head(&self, remote: &str, branch: &str) -> Result<Option<String>, DeliveryError> {
        let ref_name = format!("refs/heads/{branch}");
        let output = self.output(["ls-remote", "--heads", remote, &ref_name])?;
        Ok(output
            .split_whitespace()
            .next()
            .filter(|value| !value.is_empty())
            .map(str::to_owned))
    }

    pub fn validate_branch(&self, branch: &str) -> Result<(), DeliveryError> {
        self.status(["check-ref-format", "--branch", branch])
    }

    pub fn stage_paths(&self, paths: &[String]) -> Result<(), DeliveryError> {
        let mut args = vec!["add".to_owned(), "--".to_owned()];
        args.extend(paths.iter().cloned());
        self.status(args)
    }

    pub fn staged_paths(&self) -> Result<Vec<String>, DeliveryError> {
        let output = run_at(
            &self.root,
            ["diff", "--cached", "--name-only", "--no-renames", "-z"],
        )?;
        nul_fields(output.stdout)
    }

    pub fn has_staged_changes(&self) -> Result<bool, DeliveryError> {
        let status = Command::new("git")
            .args(["diff", "--cached", "--quiet", "--exit-code"])
            .current_dir(&self.root)
            .status()?;
        match status.code() {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            _ => Err(DeliveryError::Git("staged diff 검사에 실패했습니다".into())),
        }
    }

    pub fn commit(&self, message: &str) -> Result<String, DeliveryError> {
        self.status(["commit", "--message", message])?;
        self.head()
    }

    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool, DeliveryError> {
        let status = Command::new("git")
            .args(["merge-base", "--is-ancestor", ancestor, descendant])
            .current_dir(&self.root)
            .status()?;
        match status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(DeliveryError::Git(
                "commit ancestry 검사에 실패했습니다".into(),
            )),
        }
    }

    pub fn push_exact(
        &self,
        remote: &str,
        branch: &str,
        source_commit: &str,
        expected_remote: Option<&str>,
    ) -> Result<(), DeliveryError> {
        let target = format!("refs/heads/{branch}");
        let lease = format!(
            "--force-with-lease={}:{}",
            target,
            expected_remote.unwrap_or("")
        );
        let refspec = format!("{source_commit}:{target}");
        self.status(["push", "--porcelain", &lease, remote, &refspec])
    }
}

fn run_at<I, S>(directory: &Path, args: I) -> Result<Output, DeliveryError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .output()?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(command_error(output))
    }
}

fn output_text(output: Output) -> Result<String, DeliveryError> {
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn nul_fields(output: Vec<u8>) -> Result<Vec<String>, DeliveryError> {
    output
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .map(|field| String::from_utf8(field.to_vec()).map_err(DeliveryError::from))
        .collect()
}

fn command_error(output: Output) -> DeliveryError {
    let stderr = String::from_utf8_lossy(&output.stderr);
    DeliveryError::Git(stderr.trim().to_owned())
}
