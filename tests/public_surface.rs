use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn powershell() -> &'static str {
    "pwsh"
}

#[test]
fn secret_surface_audit_script_succeeds_for_tracked_repo_state() -> Result<()> {
    let script_path = repo_root().join("scripts").join("audit-secret-surface.ps1");
    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(&script_path)
        .current_dir(repo_root())
        .output()
        .with_context(|| format!("failed to run {}", script_path.display()))?;

    assert!(
        output.status.success(),
        "audit-secret-surface failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn public_surface_audit_script_succeeds_for_tracked_repo_state() -> Result<()> {
    let script_path = repo_root().join("scripts").join("audit-public-surface.ps1");
    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(&script_path)
        .current_dir(repo_root())
        .output()
        .with_context(|| format!("failed to run {}", script_path.display()))?;

    assert!(
        output.status.success(),
        "audit-public-surface failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn secret_surface_audit_allows_placeholder_assignments() -> Result<()> {
    let repo = tempdir()?;
    initialize_git_repo(repo.path())?;
    let script_path = repo.path().join("audit-secret-surface.ps1");
    std::fs::copy(
        repo_root().join("scripts").join("audit-secret-surface.ps1"),
        &script_path,
    )?;
    std::fs::write(
        repo.path().join("README.md"),
        "TELEGRAM_BOT_TOKEN=<YOUR_TELEGRAM_BOT_TOKEN>\nLIVE_WORKSPACE=C:/path/to/workspace\n",
    )?;
    git_add(repo.path(), "README.md")?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(&script_path)
        .current_dir(repo.path())
        .output()
        .with_context(|| format!("failed to run {}", script_path.display()))?;

    assert!(
        output.status.success(),
        "audit-secret-surface should allow placeholders: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn secret_surface_audit_rejects_live_assignments() -> Result<()> {
    let repo = tempdir()?;
    initialize_git_repo(repo.path())?;
    let script_path = repo.path().join("audit-secret-surface.ps1");
    std::fs::copy(
        repo_root().join("scripts").join("audit-secret-surface.ps1"),
        &script_path,
    )?;
    std::fs::write(
        repo.path().join("README.md"),
        concat!("LIVE_TELEGRAM_CHAT_ID", "=8642321094\n"),
    )?;
    git_add(repo.path(), "README.md")?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(&script_path)
        .current_dir(repo.path())
        .output()
        .with_context(|| format!("failed to run {}", script_path.display()))?;

    assert!(
        !output.status.success(),
        "audit-secret-surface should reject live assignments"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("tracked assignment"));

    Ok(())
}

#[test]
fn secret_surface_audit_rejects_bot_token_like_values() -> Result<()> {
    let repo = tempdir()?;
    initialize_git_repo(repo.path())?;
    let script_path = repo.path().join("audit-secret-surface.ps1");
    std::fs::copy(
        repo_root().join("scripts").join("audit-secret-surface.ps1"),
        &script_path,
    )?;
    std::fs::write(
        repo.path().join("README.md"),
        concat!(
            "Use this token: 123456789",
            ":ABCDEFGHIJKLMNOPQRSTUV1234567890\n"
        ),
    )?;
    git_add(repo.path(), "README.md")?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(&script_path)
        .current_dir(repo.path())
        .output()
        .with_context(|| format!("failed to run {}", script_path.display()))?;

    assert!(
        !output.status.success(),
        "audit-secret-surface should reject token-like values"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("token-like value"));

    Ok(())
}

#[test]
fn secret_surface_audit_rejects_telegram_bot_urls_with_embedded_tokens() -> Result<()> {
    let repo = tempdir()?;
    initialize_git_repo(repo.path())?;
    let script_path = repo.path().join("audit-secret-surface.ps1");
    std::fs::copy(
        repo_root().join("scripts").join("audit-secret-surface.ps1"),
        &script_path,
    )?;
    let token_like_value = format!("{}:{}", "123456789", "A".repeat(24));
    std::fs::write(
        repo.path().join("README.md"),
        format!(
            "Invoke-RestMethod \"https://api.telegram.org/bot{token_like_value}/getUpdates\"\n"
        ),
    )?;
    git_add(repo.path(), "README.md")?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(&script_path)
        .current_dir(repo.path())
        .output()
        .with_context(|| format!("failed to run {}", script_path.display()))?;

    assert!(
        !output.status.success(),
        "audit-secret-surface should reject Telegram bot URLs with embedded tokens"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("embedded token"));

    Ok(())
}

#[test]
fn live_planning_files_and_task_contents_stay_untracked() -> Result<()> {
    let output = Command::new("git")
        .args(["ls-files"])
        .current_dir(repo_root())
        .output()
        .context("failed to list tracked files")?;

    assert!(
        output.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tracked = String::from_utf8(output.stdout)?;
    assert!(tracked.contains("tasks/README.md"));
    assert!(!tracked.contains("tasks/backlog.example.yaml"));
    assert!(!tracked.contains("tasks/roadmap-title-ja.example.psd1"));
    assert!(!tracked.contains("tasks/ROADMAP.example.md"));
    assert!(!tracked.contains("tasks/backlog.yaml"));
    assert!(!tracked.contains("tasks/roadmap-title-ja.psd1"));
    assert!(!tracked.contains("docs/project/ROADMAP.md"));

    Ok(())
}

#[test]
fn public_surface_audit_rejects_live_planning_files_present_in_repo() -> Result<()> {
    let repo = tempdir()?;
    initialize_git_repo(repo.path())?;
    let script_path = repo.path().join("audit-public-surface.ps1");
    std::fs::copy(
        repo_root().join("scripts").join("audit-public-surface.ps1"),
        &script_path,
    )?;
    std::fs::create_dir_all(repo.path().join("tasks"))?;
    std::fs::write(
        repo.path().join("tasks").join("README.md"),
        "internal helper\n",
    )?;
    std::fs::write(
        repo.path().join("tasks").join("backlog.yaml"),
        "- id: TASK-001\n",
    )?;
    std::fs::create_dir_all(repo.path().join("scripts"))?;
    for script in [
        "audit-doc-terminology.ps1",
        "assert-release-doc-review.ps1",
        "audit-secret-surface.ps1",
        "planning-paths.ps1",
        "setup-planning.ps1",
        "sync-roadmap.ps1",
        "validate-planning.ps1",
    ] {
        std::fs::copy(
            repo_root().join("scripts").join(script),
            repo.path().join("scripts").join(script),
        )?;
    }
    for tracked_path in [
        "tasks/README.md",
        "scripts/audit-doc-terminology.ps1",
        "scripts/assert-release-doc-review.ps1",
        "scripts/audit-secret-surface.ps1",
        "scripts/planning-paths.ps1",
        "scripts/setup-planning.ps1",
        "scripts/sync-roadmap.ps1",
        "scripts/validate-planning.ps1",
    ] {
        git_add(repo.path(), tracked_path)?;
    }

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(&script_path)
        .current_dir(repo.path())
        .output()
        .with_context(|| format!("failed to run {}", script_path.display()))?;

    assert!(
        !output.status.success(),
        "audit-public-surface should reject live planning files"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("forbidden live file present in repo"));

    Ok(())
}

#[test]
fn public_surface_audit_rejects_unexpected_tracked_task_files() -> Result<()> {
    let repo = tempdir()?;
    initialize_git_repo(repo.path())?;
    let script_path = repo.path().join("audit-public-surface.ps1");
    std::fs::copy(
        repo_root().join("scripts").join("audit-public-surface.ps1"),
        &script_path,
    )?;
    std::fs::create_dir_all(repo.path().join("tasks"))?;
    std::fs::write(
        repo.path().join("tasks").join("README.md"),
        "internal helper\n",
    )?;
    std::fs::write(
        repo.path().join("tasks").join("notes.md"),
        "private notes\n",
    )?;
    std::fs::create_dir_all(repo.path().join("scripts"))?;
    for script in [
        "audit-doc-terminology.ps1",
        "assert-release-doc-review.ps1",
        "audit-secret-surface.ps1",
        "planning-paths.ps1",
        "setup-planning.ps1",
        "sync-roadmap.ps1",
        "validate-planning.ps1",
    ] {
        std::fs::copy(
            repo_root().join("scripts").join(script),
            repo.path().join("scripts").join(script),
        )?;
    }
    for tracked_path in [
        "tasks/README.md",
        "tasks/notes.md",
        "scripts/audit-doc-terminology.ps1",
        "scripts/assert-release-doc-review.ps1",
        "scripts/audit-secret-surface.ps1",
        "scripts/planning-paths.ps1",
        "scripts/setup-planning.ps1",
        "scripts/sync-roadmap.ps1",
        "scripts/validate-planning.ps1",
    ] {
        git_add(repo.path(), tracked_path)?;
    }

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(&script_path)
        .current_dir(repo.path())
        .output()
        .with_context(|| format!("failed to run {}", script_path.display()))?;

    assert!(
        !output.status.success(),
        "audit-public-surface should reject tracked task artifacts"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unexpected tracked task file: tasks/notes.md"));

    Ok(())
}

#[test]
fn public_surface_audit_rejects_live_planning_files_anywhere_in_repo() -> Result<()> {
    let repo = tempdir()?;
    initialize_git_repo(repo.path())?;
    let script_path = repo.path().join("audit-public-surface.ps1");
    std::fs::copy(
        repo_root().join("scripts").join("audit-public-surface.ps1"),
        &script_path,
    )?;
    std::fs::create_dir_all(repo.path().join("tasks"))?;
    std::fs::write(
        repo.path().join("tasks").join("README.md"),
        "internal helper\n",
    )?;
    std::fs::create_dir_all(repo.path().join("private-planning"))?;
    std::fs::write(
        repo.path().join("private-planning").join("backlog.yaml"),
        "- id: TASK-001\n",
    )?;
    std::fs::create_dir_all(repo.path().join("scripts"))?;
    for script in [
        "audit-doc-terminology.ps1",
        "assert-release-doc-review.ps1",
        "audit-secret-surface.ps1",
        "planning-paths.ps1",
        "setup-planning.ps1",
        "sync-roadmap.ps1",
        "validate-planning.ps1",
    ] {
        std::fs::copy(
            repo_root().join("scripts").join(script),
            repo.path().join("scripts").join(script),
        )?;
    }
    for tracked_path in [
        "tasks/README.md",
        "scripts/audit-doc-terminology.ps1",
        "scripts/assert-release-doc-review.ps1",
        "scripts/audit-secret-surface.ps1",
        "scripts/planning-paths.ps1",
        "scripts/setup-planning.ps1",
        "scripts/sync-roadmap.ps1",
        "scripts/validate-planning.ps1",
    ] {
        git_add(repo.path(), tracked_path)?;
    }

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(&script_path)
        .current_dir(repo.path())
        .output()
        .with_context(|| format!("failed to run {}", script_path.display()))?;

    assert!(
        !output.status.success(),
        "audit-public-surface should reject in-repo planning roots"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("forbidden live file present in repo: private-planning"));

    Ok(())
}

fn initialize_git_repo(path: &std::path::Path) -> Result<()> {
    let output = Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .context("failed to initialize git repo")?;
    assert!(
        output.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

fn git_add(path: &std::path::Path, file: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["add", file])
        .current_dir(path)
        .output()
        .with_context(|| format!("failed to add {file}"))?;
    assert!(
        output.status.success(),
        "git add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}
