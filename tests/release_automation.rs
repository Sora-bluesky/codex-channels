use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn powershell() -> &'static str {
    "pwsh"
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

#[test]
fn generate_release_notes_prefers_history_entries() -> Result<()> {
    let temp = TempDir::new()?;
    let history_path = temp.path().join("release-history.psd1");
    let output_path = temp.path().join("release-body.md");

    write_file(
        &history_path,
        r#"@{
    Releases = @(
        @{
            Version = "0.1.0"
            Commit = "1111111111111111111111111111111111111111"
            Title = "Foundation"
            Notes = @("Initial release")
        }
        @{
            Version = "0.1.1"
            Commit = "2222222222222222222222222222222222222222"
            Title = "Second"
            Notes = @("Adds service commands", "Adds release notes")
        }
    )
}"#,
    )?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(
            repo_root()
                .join("scripts")
                .join("generate-release-notes.ps1"),
        )
        .arg("-Version")
        .arg("v0.1.1")
        .arg("-HistoryPath")
        .arg(&history_path)
        .arg("-OutputPath")
        .arg(&output_path)
        .arg("-Repository")
        .arg("owner/repo")
        .output()
        .context("failed to run generate-release-notes.ps1")?;

    assert!(
        output.status.success(),
        "generate-release-notes failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let body = fs::read_to_string(&output_path)?;
    assert!(body.contains("Adds service commands"));
    assert!(body.contains("Adds release notes"));
    assert!(body.contains("https://github.com/owner/repo/compare/v0.1.0...v0.1.1"));

    Ok(())
}

#[test]
fn generate_release_notes_falls_back_to_planning_titles() -> Result<()> {
    let temp = TempDir::new()?;
    let history_path = temp.path().join("release-history.psd1");
    let backlog_path = temp.path().join("backlog.yaml");
    let output_path = temp.path().join("release-body.md");

    write_file(
        &history_path,
        r#"@{
    Releases = @(
        @{
            Version = "0.1.0"
            Commit = "1111111111111111111111111111111111111111"
            Title = "Foundation"
            Notes = @("Initial release")
        }
    )
}"#,
    )?;
    write_file(
        &backlog_path,
        r#"# === v0.1.1: Operator controls ===
- id: TASK-001
    title: Add Telegram control commands and service management
    status: done
    priority: P0
    target_version: v0.1.1
    repo: remotty
"#,
    )?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(
            repo_root()
                .join("scripts")
                .join("generate-release-notes.ps1"),
        )
        .arg("-Version")
        .arg("0.1.1")
        .arg("-HistoryPath")
        .arg(&history_path)
        .arg("-BacklogPath")
        .arg(&backlog_path)
        .arg("-OutputPath")
        .arg(&output_path)
        .arg("-Repository")
        .arg("owner/repo")
        .output()
        .context("failed to run generate-release-notes.ps1")?;

    assert!(
        output.status.success(),
        "generate-release-notes failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let body = fs::read_to_string(&output_path)?;
    assert!(body.contains("Add Telegram control commands and service management"));
    assert!(body.contains("https://github.com/owner/repo/compare/v0.1.0...v0.1.1"));

    Ok(())
}

#[test]
fn generate_release_notes_uses_backlog_env_override_when_no_argument_is_passed() -> Result<()> {
    let temp = TempDir::new()?;
    let history_path = temp.path().join("release-history.psd1");
    let planning_root = temp.path().join("planning-root");
    let explicit_backlog_path = temp.path().join("custom-backlog.yaml");
    let output_path = temp.path().join("release-body.md");

    write_file(
        &history_path,
        r#"@{
    Releases = @(
        @{
            Version = "0.1.0"
            Commit = "1111111111111111111111111111111111111111"
            Title = "Foundation"
            Notes = @("Initial release")
        }
    )
}"#,
    )?;
    write_file(
        &explicit_backlog_path,
        r#"# === v0.1.1: Operator controls ===
- id: TASK-001
    title: Use explicit backlog override
    status: done
    priority: P0
    target_version: v0.1.1
    repo: remotty
"#,
    )?;
    write_file(
        &planning_root.join("backlog.yaml"),
        r#"# === v0.1.1: Wrong source ===
- id: TASK-999
    title: Wrong backlog source
    status: done
    priority: P0
    target_version: v0.1.1
    repo: remotty
"#,
    )?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(
            repo_root()
                .join("scripts")
                .join("generate-release-notes.ps1"),
        )
        .arg("-Version")
        .arg("0.1.1")
        .arg("-HistoryPath")
        .arg(&history_path)
        .arg("-OutputPath")
        .arg(&output_path)
        .env("REMOTTY_PLANNING_ROOT", &planning_root)
        .env("REMOTTY_BACKLOG_PATH", &explicit_backlog_path)
        .output()
        .context("failed to run generate-release-notes.ps1 with env override")?;

    assert!(
        output.status.success(),
        "generate-release-notes failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let body = fs::read_to_string(&output_path)?;
    assert!(body.contains("Use explicit backlog override"));
    assert!(!body.contains("Wrong backlog source"));

    Ok(())
}

#[test]
fn bump_version_sync_only_updates_version_sources() -> Result<()> {
    let temp = TempDir::new()?;
    let cargo_toml_path = temp.path().join("Cargo.toml");
    let version_path = temp.path().join("VERSION");

    write_file(
        &cargo_toml_path,
        r#"[package]
name = "remotty"
version = "0.1.0"
edition = "2024"
"#,
    )?;
    write_file(&version_path, "0.1.0")?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(repo_root().join("scripts").join("bump-version.ps1"))
        .arg("-RepoRoot")
        .arg(temp.path())
        .arg("-Version")
        .arg("0.1.8")
        .arg("-SyncOnly")
        .output()
        .context("failed to run bump-version.ps1")?;

    assert!(
        output.status.success(),
        "bump-version failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cargo_toml = fs::read_to_string(&cargo_toml_path)?;
    assert!(cargo_toml.contains("version = \"0.1.8\""));
    assert_eq!(fs::read_to_string(&version_path)?, "0.1.8");

    Ok(())
}

#[test]
fn release_doc_review_gate_requires_review_file() -> Result<()> {
    let temp = TempDir::new()?;
    let missing_review = temp.path().join("missing-review.psd1");

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(
            repo_root()
                .join("scripts")
                .join("assert-release-doc-review.ps1"),
        )
        .arg("-Version")
        .arg("0.1.18")
        .arg("-ReviewPath")
        .arg(&missing_review)
        .output()
        .context("failed to run assert-release-doc-review.ps1")?;

    assert!(
        !output.status.success(),
        "doc review gate unexpectedly passed"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Release documentation review is required"));

    Ok(())
}

#[test]
fn release_doc_review_gate_accepts_opus_review_record() -> Result<()> {
    let temp = TempDir::new()?;
    let review_path = temp.path().join("v0.1.18.psd1");
    write_file(
        &review_path,
        r#"@{
    Version = "v0.1.18"
    EnglishReviewStatus = "approved"
    JapaneseReviewStatus = "approved"
    JapaneseReviewerModel = "claude-opus-4-7"
    ReviewedDocs = @("README.md", "README.ja.md", "plugins/remotty/README.md")
    Notes = "English and Japanese public docs were reviewed before release."
}"#,
    )?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(
            repo_root()
                .join("scripts")
                .join("assert-release-doc-review.ps1"),
        )
        .arg("-Version")
        .arg("v0.1.18")
        .arg("-ReviewPath")
        .arg(&review_path)
        .output()
        .context("failed to run assert-release-doc-review.ps1")?;

    assert!(
        output.status.success(),
        "doc review gate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn release_doc_review_gate_rejects_non_opus_japanese_review() -> Result<()> {
    let temp = TempDir::new()?;
    let review_path = temp.path().join("v0.1.18.psd1");
    write_file(
        &review_path,
        r#"@{
    Version = "v0.1.18"
    EnglishReviewStatus = "approved"
    JapaneseReviewStatus = "approved"
    JapaneseReviewerModel = "claude-sonnet-4-5"
    ReviewedDocs = @("README.md", "README.ja.md")
    Notes = "Japanese docs were checked."
}"#,
    )?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(
            repo_root()
                .join("scripts")
                .join("assert-release-doc-review.ps1"),
        )
        .arg("-Version")
        .arg("v0.1.18")
        .arg("-ReviewPath")
        .arg(&review_path)
        .output()
        .context("failed to run assert-release-doc-review.ps1")?;

    assert!(
        !output.status.success(),
        "doc review gate unexpectedly accepted non-Opus review"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("claude-opus-4-7"));

    Ok(())
}

#[test]
fn release_workflow_runs_doc_review_gate_before_publishing() -> Result<()> {
    let workflow = fs::read_to_string(
        repo_root()
            .join(".github")
            .join("workflows")
            .join("release.yml"),
    )?;
    let gate = "scripts/assert-release-doc-review.ps1";
    let publish = "softprops/action-gh-release";

    let gate_index = workflow
        .find(gate)
        .context("release workflow must run release documentation review gate")?;
    let publish_index = workflow
        .find(publish)
        .context("release workflow must publish GitHub release assets")?;

    assert!(
        gate_index < publish_index,
        "release documentation review gate must run before release publishing"
    );
    assert!(
        workflow.contains(".github/release-doc-reviews/${{ github.ref_name }}.psd1"),
        "release workflow must use the committed public review record"
    );

    Ok(())
}

#[test]
fn bump_version_fails_when_planning_inputs_are_missing() -> Result<()> {
    let temp = TempDir::new()?;
    let planning_root = temp.path().join("planning-root");
    let cargo_toml_path = temp.path().join("Cargo.toml");
    let version_path = temp.path().join("VERSION");

    fs::create_dir_all(&planning_root)?;
    write_file(
        &cargo_toml_path,
        r#"[package]
name = "remotty"
version = "0.1.0"
edition = "2024"
"#,
    )?;
    write_file(&version_path, "0.1.0")?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(repo_root().join("scripts").join("bump-version.ps1"))
        .arg("-RepoRoot")
        .arg(temp.path())
        .arg("-Version")
        .arg("0.1.8")
        .env("REMOTTY_PLANNING_ROOT", &planning_root)
        .output()
        .context("failed to run bump-version.ps1 with missing planning inputs")?;

    assert!(
        !output.status.success(),
        "bump-version unexpectedly succeeded"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("backlog.yaml not found"));
    assert!(fs::read_to_string(&version_path)? == "0.1.0");

    let cargo_toml = fs::read_to_string(&cargo_toml_path)?;
    assert!(cargo_toml.contains("version = \"0.1.0\""));

    Ok(())
}

#[test]
fn release_preflight_allows_missing_title_map_when_backlog_exists() -> Result<()> {
    let temp = TempDir::new()?;
    let backlog_path = temp.path().join("backlog.yaml");
    let missing_title_path = temp.path().join("roadmap-title-ja.psd1");

    write_file(
        &backlog_path,
        r#"# === v0.1.8: Release ===
- id: TASK-001
    title: Keep backlog available
    status: done
    priority: P0
    target_version: v0.1.8
    repo: remotty
"#,
    )?;

    let script = format!(
        ". '{}' ; Assert-ReleasePlanningInputsExist -BacklogPath '{}' -RoadmapTitleJaPath '{}' ; 'ok'",
        repo_root()
            .join("scripts")
            .join("release-common.ps1")
            .display(),
        backlog_path.display(),
        missing_title_path.display(),
    );

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-Command")
        .arg(&script)
        .output()
        .context("failed to run release preflight assertion")?;

    assert!(
        output.status.success(),
        "release preflight unexpectedly failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ok");

    Ok(())
}

#[test]
fn bump_version_fails_when_title_map_is_invalid() -> Result<()> {
    let temp = TempDir::new()?;
    let planning_root = temp.path().join("planning-root");
    let cargo_toml_path = temp.path().join("Cargo.toml");
    let version_path = temp.path().join("VERSION");

    fs::create_dir_all(&planning_root)?;
    write_file(
        &cargo_toml_path,
        r#"[package]
name = "remotty"
version = "0.1.0"
edition = "2024"
"#,
    )?;
    write_file(&version_path, "0.1.0")?;
    write_file(
        &planning_root.join("backlog.yaml"),
        r#"# === v0.1.8: Release ===
- id: TASK-001
    title: Keep backlog available
    status: done
    priority: P0
    target_version: v0.1.8
    repo: remotty
"#,
    )?;
    write_file(
        &planning_root.join("roadmap-title-ja.psd1"),
        "@{\nVersionTitles =\n",
    )?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(repo_root().join("scripts").join("bump-version.ps1"))
        .arg("-RepoRoot")
        .arg(temp.path())
        .arg("-Version")
        .arg("0.1.8")
        .env("REMOTTY_PLANNING_ROOT", &planning_root)
        .output()
        .context("failed to run bump-version.ps1 with invalid title map")?;

    assert!(
        !output.status.success(),
        "bump-version unexpectedly succeeded"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("roadmap-title-ja.psd1 is invalid"));
    assert_eq!(fs::read_to_string(&version_path)?, "0.1.0");

    let cargo_toml = fs::read_to_string(&cargo_toml_path)?;
    assert!(cargo_toml.contains("version = \"0.1.0\""));

    Ok(())
}

#[test]
fn bump_version_fails_when_backlog_is_invalid() -> Result<()> {
    let temp = TempDir::new()?;
    let planning_root = temp.path().join("planning-root");
    let cargo_toml_path = temp.path().join("Cargo.toml");
    let version_path = temp.path().join("VERSION");

    fs::create_dir_all(&planning_root)?;
    write_file(
        &cargo_toml_path,
        r#"[package]
name = "remotty"
version = "0.1.0"
edition = "2024"
"#,
    )?;
    write_file(&version_path, "0.1.0")?;
    write_file(
        &planning_root.join("backlog.yaml"),
        r#"# === v0.1.8: Release ===
- id: TASK-001
    title: Invalid backlog
    status: progress
    priority: P0
    target_version: 0.1.8
    repo: remotty
"#,
    )?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(repo_root().join("scripts").join("bump-version.ps1"))
        .arg("-RepoRoot")
        .arg(temp.path())
        .arg("-Version")
        .arg("0.1.8")
        .env("REMOTTY_PLANNING_ROOT", &planning_root)
        .output()
        .context("failed to run bump-version.ps1 with invalid backlog")?;

    assert!(
        !output.status.success(),
        "bump-version unexpectedly succeeded"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("planning validation failed"));
    assert!(stderr.contains("invalid status"));
    assert_eq!(fs::read_to_string(&version_path)?, "0.1.0");

    let cargo_toml = fs::read_to_string(&cargo_toml_path)?;
    assert!(cargo_toml.contains("version = \"0.1.0\""));

    Ok(())
}
