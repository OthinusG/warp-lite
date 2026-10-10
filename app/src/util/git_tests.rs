use std::path::Path;

use command::r#async::Command;
use command::Stdio;
use tempfile::TempDir;

use super::{detect_current_branch, detect_current_branch_display};

#[tokio::test]
async fn remove_worktree_preserves_protected_checkouts_and_branch() {
    let (_repo, repository) = init_repo().await;
    let checkouts = tempfile::tempdir().unwrap();
    let checkout = checkouts.path().join("linked");
    git(&repository, &["worktree", "add", "-b", "linked", checkout.to_str().unwrap()]).await;
    assert!(super::remove_local_worktree(&repository, &repository, &[]).await.is_err());
    assert!(super::remove_local_worktree(&repository, &checkout, &[checkout.clone()]).await.is_err());
    git(&repository, &["worktree", "lock", checkout.to_str().unwrap()]).await;
    assert!(super::remove_local_worktree(&repository, &checkout, &[]).await.is_err());
    git(&repository, &["worktree", "unlock", checkout.to_str().unwrap()]).await;
    std::fs::write(checkout.join("untracked.txt"), "keep").unwrap();
    assert!(super::remove_local_worktree(&repository, &checkout, &[]).await.is_err());
    assert_eq!(std::fs::read_to_string(checkout.join("untracked.txt")).unwrap(), "keep");
    std::fs::remove_file(checkout.join("untracked.txt")).unwrap();
    std::fs::write(checkout.join(".gitignore"), "ignored.txt\n").unwrap();
    git(&checkout, &["add", ".gitignore"]).await;
    git(&checkout, &["commit", "-m", "ignore fixture"]).await;
    std::fs::write(checkout.join("ignored.txt"), "keep ignored").unwrap();
    assert!(super::remove_local_worktree(&repository, &checkout, &[]).await.is_err());
    std::fs::remove_file(checkout.join("ignored.txt")).unwrap();
    std::fs::write(checkout.join(".gitignore"), "changed\n").unwrap();
    assert!(super::remove_local_worktree(&repository, &checkout, &[]).await.is_err());
    git(&checkout, &["checkout", "--", ".gitignore"]).await;
    super::remove_local_worktree(&repository, &checkout, &[]).await.unwrap();
    assert!(!checkout.exists());
    assert!(git(&repository, &["branch", "--list", "linked"]).await.contains("linked"));
}

/// Helper: run a git command inside the given repo directory.
async fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("failed to run git");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// Creates a temp git repo with one commit and returns `(dir_handle, repo_path)`.
async fn init_repo() -> (TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let path = dir.path().to_path_buf();

    git(&path, &["init", "-b", "main"]).await;
    git(&path, &["config", "user.email", "test@test.com"]).await;
    git(&path, &["config", "user.name", "Test"]).await;
    git(&path, &["commit", "--allow-empty", "-m", "initial"]).await;

    (dir, path)
}

#[tokio::test]
async fn on_normal_branch_returns_branch_name() {
    let (_dir, repo) = init_repo().await;
    git(&repo, &["checkout", "-b", "feature-xyz"]).await;

    assert_eq!(detect_current_branch(&repo).await.unwrap(), "feature-xyz");
    assert_eq!(
        detect_current_branch_display(&repo).await.unwrap(),
        "feature-xyz"
    );
}

#[tokio::test]
async fn detached_head_raw_returns_head() {
    let (_dir, repo) = init_repo().await;
    git(&repo, &["checkout", "--detach", "HEAD"]).await;

    assert_eq!(detect_current_branch(&repo).await.unwrap(), "HEAD");
}

#[tokio::test]
async fn detached_head_display_returns_short_sha() {
    let (_dir, repo) = init_repo().await;
    let full_sha = git(&repo, &["rev-parse", "HEAD"]).await;
    git(&repo, &["checkout", "--detach", "HEAD"]).await;

    let result = detect_current_branch_display(&repo).await.unwrap();

    assert_ne!(
        result, "HEAD",
        "display variant should not return literal HEAD"
    );
    assert!(
        full_sha.starts_with(&result),
        "expected {full_sha} to start with {result}"
    );
}

#[tokio::test]
async fn detached_tag_display_returns_short_sha() {
    let (_dir, repo) = init_repo().await;
    git(&repo, &["tag", "v1.0"]).await;
    git(&repo, &["checkout", "v1.0"]).await;

    let full_sha = git(&repo, &["rev-parse", "HEAD"]).await;
    let result = detect_current_branch_display(&repo).await.unwrap();

    assert_ne!(result, "HEAD");
    assert!(
        full_sha.starts_with(&result),
        "expected {full_sha} to start with {result}"
    );
}
