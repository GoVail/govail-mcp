use std::fs;
use std::path::Path;
use std::process::Command;

use govail_mcp::developer::DeliveryService;
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    repository: std::path::PathBuf,
    remote: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let remote = temp.path().join("remote.git");
        let repository = temp.path().join("work");
        git(temp.path(), ["init", "--bare", remote.to_str().unwrap()]);
        git(temp.path(), ["init", repository.to_str().unwrap()]);
        git(
            &repository,
            ["config", "user.email", "developer@example.com"],
        );
        git(&repository, ["config", "user.name", "Developer Test"]);
        fs::write(repository.join("README.md"), "initial\n").unwrap();
        git(&repository, ["add", "--", "README.md"]);
        git(&repository, ["commit", "-m", "initial"]);
        git(
            &repository,
            ["remote", "add", "origin", remote.to_str().unwrap()],
        );
        Self {
            _temp: temp,
            repository,
            remote,
        }
    }

    fn service(&self) -> DeliveryService {
        DeliveryService::open(&self.repository).unwrap()
    }

    fn commit_change(&self) -> String {
        fs::write(self.repository.join("feature.txt"), "feature\n").unwrap();
        self.service()
            .commit_allowlisted(&["feature.txt".into()], "feat: add feature")
            .unwrap()
    }

    fn verify(&self) -> String {
        self.service()
            .verify("repository-clean", "/usr/bin/true", &[])
            .unwrap()
            .id
    }
}

#[test]
fn approved_exact_commit_is_pushed_once() {
    let fixture = Fixture::new();
    let commit = fixture.commit_change();
    let verification = fixture.verify();
    let proposal = fixture
        .service()
        .propose_push("origin", "feature/delivery", &verification)
        .unwrap();
    let approval = fixture
        .service()
        .approve(&proposal.id, "local:test-user", 600)
        .unwrap();

    fixture
        .service()
        .push(&approval.claims.approval_id)
        .unwrap();
    let remote_commit = output(
        fixture._temp.path(),
        [
            "--git-dir",
            fixture.remote.to_str().unwrap(),
            "rev-parse",
            "refs/heads/feature/delivery",
        ],
    );
    assert_eq!(remote_commit, commit);

    let reused = fixture.service().push(&approval.claims.approval_id);
    assert!(reused
        .unwrap_err()
        .to_string()
        .contains("already been consumed"));
}

#[test]
fn local_head_drift_invalidates_approval() {
    let fixture = Fixture::new();
    fixture.commit_change();
    let verification = fixture.verify();
    let proposal = fixture
        .service()
        .propose_push("origin", "feature/delivery", &verification)
        .unwrap();
    let approval = fixture
        .service()
        .approve(&proposal.id, "local:test-user", 600)
        .unwrap();

    fs::write(fixture.repository.join("drift.txt"), "drift\n").unwrap();
    fixture
        .service()
        .commit_allowlisted(&["drift.txt".into()], "test: create drift")
        .unwrap();

    let result = fixture.service().push(&approval.claims.approval_id);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("local HEAD changed"));
}

#[test]
fn protected_branch_and_path_traversal_are_denied() {
    let fixture = Fixture::new();
    fixture.commit_change();
    let verification = fixture.verify();
    let protected = fixture
        .service()
        .propose_push("origin", "main", &verification);
    assert!(protected
        .unwrap_err()
        .to_string()
        .contains("protected branch"));

    let traversal = fixture
        .service()
        .commit_allowlisted(&["../outside".into()], "bad");
    assert!(traversal
        .unwrap_err()
        .to_string()
        .contains("without traversal"));
}

#[test]
fn preexisting_staged_change_is_never_mixed_into_allowlisted_commit() {
    let fixture = Fixture::new();
    fs::write(fixture.repository.join("unrelated.txt"), "unrelated\n").unwrap();
    git(&fixture.repository, ["add", "--", "unrelated.txt"]);
    fs::write(fixture.repository.join("wanted.txt"), "wanted\n").unwrap();

    let result = fixture
        .service()
        .commit_allowlisted(&["wanted.txt".into()], "feat: wanted only");
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("already contains staged changes"));
}

#[test]
fn inspection_preserves_dotfiles_and_whitespace_in_paths() {
    let fixture = Fixture::new();
    fs::write(fixture.repository.join(".hidden file"), "change\n").unwrap();

    let inspection = fixture.service().inspect().unwrap();
    assert_eq!(inspection.changed_paths, vec![".hidden file"]);
}

#[test]
fn remote_drift_invalidates_approval_before_push() {
    let fixture = Fixture::new();
    fixture.commit_change();
    let verification = fixture.verify();
    let proposal = fixture
        .service()
        .propose_push("origin", "feature/delivery", &verification)
        .unwrap();
    let approval = fixture
        .service()
        .approve(&proposal.id, "local:test-user", 600)
        .unwrap();
    let initial = output(&fixture.repository, ["rev-parse", "HEAD^"]);
    let competing_refspec = format!("{initial}:refs/heads/feature/delivery");
    git(&fixture.repository, ["push", "origin", &competing_refspec]);

    let result = fixture.service().push(&approval.claims.approval_id);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("remote HEAD changed"));
}

#[test]
fn tampered_approval_receipt_is_rejected() {
    let fixture = Fixture::new();
    fixture.commit_change();
    let verification = fixture.verify();
    let proposal = fixture
        .service()
        .propose_push("origin", "feature/delivery", &verification)
        .unwrap();
    let approval = fixture
        .service()
        .approve(&proposal.id, "local:test-user", 600)
        .unwrap();
    let approval_path = fixture
        .repository
        .join(".git/govail/developer-delivery/approvals")
        .join(format!("{}.json", approval.claims.approval_id));
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&approval_path).unwrap()).unwrap();
    value["claims"]["approver"] = serde_json::json!("local:attacker");
    fs::write(&approval_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();

    let result = fixture.service().push(&approval.claims.approval_id);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("signature is invalid"));
}

fn git<const N: usize>(directory: &Path, args: [&str; N]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(directory)
        .status()
        .unwrap();
    assert!(status.success());
}

fn output<const N: usize>(directory: &Path, args: [&str; N]) -> String {
    let result = Command::new("git")
        .args(args)
        .current_dir(directory)
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout).unwrap().trim().to_owned()
}
