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

fn normalize_roadmap_timestamp(content: &str) -> String {
    content
        .replace("\r\n", "\n")
        .lines()
        .map(|line| {
            if line.starts_with("> 最終同期: ") {
                String::from("> 最終同期: YYYY-MM-DD HH:mm (+09:00)")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_validate_planning(backlog_path: &Path, title_path: &Path) -> Result<std::process::Output> {
    Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(repo_root().join("scripts").join("validate-planning.ps1"))
        .arg("-BacklogPath")
        .arg(backlog_path)
        .arg("-RoadmapTitleJaPath")
        .arg(title_path)
        .output()
        .context("failed to run validate-planning.ps1")
}

#[test]
fn sync_roadmap_generates_grouped_japanese_view() -> Result<()> {
    let temp = TempDir::new()?;
    let backlog_path = temp.path().join("backlog.yaml");
    let roadmap_path = temp.path().join("ROADMAP.md");
    let title_path = temp.path().join("roadmap-title-ja.psd1");

    write_file(
        &backlog_path,
        r#"# === v0.1.0: Bootstrap foundation ===
- id: TASK-001
    title: Create bridge foundation
    status: done
    priority: P0
    target_version: v0.1.0
    repo: remotty

# === v0.1.1: Service commands ===
- id: TASK-002
    title: Add Windows service management commands
    status: active
    priority: P1
    target_version: v0.1.1
    repo: remotty

# === v0.1.2: Completion checks ===
- id: TASK-003
    title: Implement completion checks follow-up flow
    status: backlog
    priority: P0
    target_version: v0.1.2
    repo: remotty
"#,
    )?;

    write_file(
        &title_path,
        r#"@{
    VersionTitles = @{
        "v0.1.0" = "基盤の立ち上げ"
        "v0.1.1" = "サービス運用の追加"
        "v0.1.2" = "確認フローの追加"
    }
    TaskTitles = @{
        "TASK-002" = "Windows サービス管理コマンドを追加"
    }
}"#,
    )?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(repo_root().join("scripts").join("sync-roadmap.ps1"))
        .arg("-BacklogPath")
        .arg(&backlog_path)
        .arg("-RoadmapPath")
        .arg(&roadmap_path)
        .arg("-RoadmapTitleJaPath")
        .arg(&title_path)
        .output()
        .context("failed to run sync-roadmap.ps1")?;

    assert!(
        output.status.success(),
        "sync-roadmap failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let roadmap = fs::read_to_string(&roadmap_path)?;
    let expected = fs::read_to_string(
        repo_root()
            .join("tests")
            .join("fixtures")
            .join("roadmap_snapshot.md"),
    )?;
    assert_eq!(
        normalize_roadmap_timestamp(&roadmap),
        normalize_roadmap_timestamp(&expected)
    );
    assert!(roadmap.contains("# ロードマップ"));
    assert!(roadmap.contains("### v0.1.0: 基盤の立ち上げ"));
    assert!(roadmap.contains("### v0.1.1: サービス運用の追加"));
    assert!(roadmap.contains("### v0.1.2: 確認フローの追加"));
    assert!(roadmap.contains("| v0.1.0 | 1 |"));
    assert!(roadmap.contains("| v0.1.1 | 1 |"));
    assert!(roadmap.contains("| v0.1.2 | 1 |"));
    assert!(roadmap.contains("Windows サービス管理コマンドを追加"));
    assert!(roadmap.contains("completion checks follow-up flow を実装"));
    let foundation_index = roadmap.find("### v0.1.0: 基盤の立ち上げ").unwrap();
    let service_index = roadmap.find("### v0.1.1: サービス運用の追加").unwrap();
    let checks_index = roadmap.find("### v0.1.2: 確認フローの追加").unwrap();
    assert!(foundation_index < service_index);
    assert!(service_index < checks_index);
    assert!(
        roadmap.contains("[====================] 100% (1/1)") || roadmap.contains("[==========")
    );

    Ok(())
}

#[test]
fn planning_paths_prefers_env_override() -> Result<()> {
    let temp = TempDir::new()?;
    let planning_root = temp.path().join("planning-root");
    let expected = planning_root.join("ROADMAP.md");
    fs::create_dir_all(&planning_root)?;
    fs::write(&expected, "# existing external roadmap\n")?;

    let script = format!(
        ". '{}' ; Resolve-RemottyPlanningFilePath -RepoRoot '{}' -LocalRelativePath 'tasks/ROADMAP.md' -EnvironmentVariable 'REMOTTY_ROADMAP_PATH' -DefaultFileName 'ROADMAP.md'",
        repo_root()
            .join("scripts")
            .join("planning-paths.ps1")
            .display(),
        repo_root().display()
    );

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-Command")
        .arg(script)
        .env("REMOTTY_PLANNING_ROOT", &planning_root)
        .output()
        .context("failed to run planning-paths.ps1")?;

    assert!(
        output.status.success(),
        "planning-paths failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let resolved = String::from_utf8(output.stdout)?.trim().to_owned();
    assert_eq!(resolved, expected.display().to_string());

    Ok(())
}

#[test]
fn planning_paths_prefers_shortest_matching_root() -> Result<()> {
    let temp = TempDir::new()?;
    let preferred_root = temp.path().join("project").join("remotty").join("planning");
    let duplicate_root = temp
        .path()
        .join("very")
        .join("deep")
        .join("project")
        .join("remotty")
        .join("planning");

    fs::create_dir_all(&preferred_root)?;
    fs::create_dir_all(&duplicate_root)?;
    for root in [&preferred_root, &duplicate_root] {
        fs::write(root.join("backlog.yaml"), "- id: TASK-001\n")?;
        fs::write(root.join("ROADMAP.md"), "# roadmap\n")?;
        fs::write(root.join("roadmap-title-ja.psd1"), "@{\n}\n")?;
    }

    let script = format!(
        ". '{}' ; Get-RemottyDefaultPlanningRoot",
        repo_root()
            .join("scripts")
            .join("planning-paths.ps1")
            .display()
    );

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-Command")
        .arg(script)
        .env("USERPROFILE", temp.path())
        .env("LOCALAPPDATA", temp.path().join("LocalAppData"))
        .output()
        .context("failed to run planning-paths.ps1")?;

    assert!(
        output.status.success(),
        "planning-paths failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let resolved = fs::canonicalize(String::from_utf8(output.stdout)?.trim())?;
    let expected = fs::canonicalize(&preferred_root)?;
    assert_eq!(resolved, expected);

    Ok(())
}

#[test]
fn setup_planning_creates_live_files_and_marker() -> Result<()> {
    let temp = TempDir::new()?;
    let planning_root = temp.path().join("planning-root");
    let marker_path = temp.path().join("planning-root.txt");

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(repo_root().join("scripts").join("setup-planning.ps1"))
        .arg("-PlanningRoot")
        .arg(&planning_root)
        .arg("-MarkerPath")
        .arg(&marker_path)
        .output()
        .context("failed to run setup-planning.ps1")?;

    assert!(
        output.status.success(),
        "setup-planning failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        fs::read_to_string(&marker_path)?,
        planning_root.display().to_string()
    );
    assert!(planning_root.join("backlog.yaml").exists());
    assert!(planning_root.join("roadmap-title-ja.psd1").exists());
    assert!(planning_root.join("ROADMAP.md").exists());

    let roadmap = fs::read_to_string(planning_root.join("ROADMAP.md"))?;
    assert!(roadmap.contains("# ロードマップ"));
    assert!(roadmap.contains("初期マイルストーンを定義する"));

    Ok(())
}

#[test]
fn setup_planning_preserves_existing_live_files() -> Result<()> {
    let temp = TempDir::new()?;
    let planning_root = temp.path().join("planning-root");
    let marker_path = temp.path().join("planning-root.txt");
    fs::create_dir_all(&planning_root)?;

    let existing_backlog = planning_root.join("backlog.yaml");
    write_file(
        &existing_backlog,
        "# === v9.9.9: Custom ===\n- id: TASK-999\n    title: Keep custom backlog\n    status: active\n    priority: P0\n    target_version: v9.9.9\n    repo: remotty\n",
    )?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(repo_root().join("scripts").join("setup-planning.ps1"))
        .arg("-PlanningRoot")
        .arg(&planning_root)
        .arg("-MarkerPath")
        .arg(&marker_path)
        .output()
        .context("failed to run setup-planning.ps1")?;

    assert!(
        output.status.success(),
        "setup-planning failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let backlog = fs::read_to_string(&existing_backlog)?;
    assert!(backlog.contains("Keep custom backlog"));

    let roadmap = fs::read_to_string(planning_root.join("ROADMAP.md"))?;
    assert!(roadmap.contains("Keep custom backlog"));

    Ok(())
}

#[test]
fn setup_planning_does_not_update_marker_when_sync_fails() -> Result<()> {
    let temp = TempDir::new()?;
    let planning_root = temp.path().join("planning-root");
    let marker_path = temp.path().join("planning-root.txt");
    let previous_root = temp.path().join("previous-root");
    fs::create_dir_all(&planning_root)?;
    fs::write(&marker_path, previous_root.display().to_string())?;
    write_file(
        &planning_root.join("roadmap-title-ja.psd1"),
        "@{\nVersionTitles =\n",
    )?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(repo_root().join("scripts").join("setup-planning.ps1"))
        .arg("-PlanningRoot")
        .arg(&planning_root)
        .arg("-MarkerPath")
        .arg(&marker_path)
        .output()
        .context("failed to run setup-planning.ps1")?;

    assert!(
        !output.status.success(),
        "setup-planning unexpectedly succeeded: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(&marker_path)?,
        previous_root.display().to_string()
    );

    Ok(())
}

#[test]
fn setup_planning_rejects_repo_internal_planning_root() -> Result<()> {
    let temp = TempDir::new()?;
    let marker_path = temp.path().join("planning-root.txt");
    let repo_internal_root = repo_root().join("private-planning");

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(repo_root().join("scripts").join("setup-planning.ps1"))
        .arg("-PlanningRoot")
        .arg(&repo_internal_root)
        .arg("-MarkerPath")
        .arg(&marker_path)
        .output()
        .context("failed to run setup-planning.ps1 with repo-internal root")?;

    assert!(
        !output.status.success(),
        "setup-planning unexpectedly accepted repo-internal root"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Planning root must stay outside the repository"),
        "stderr did not mention repo boundary: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!marker_path.exists());

    Ok(())
}

#[test]
fn sync_roadmap_fails_when_backlog_is_missing() -> Result<()> {
    let temp = TempDir::new()?;
    let planning_root = temp.path().join("planning-root");
    fs::create_dir_all(&planning_root)?;
    write_file(
        &planning_root.join("roadmap-title-ja.psd1"),
        "@{\n    VersionTitles = @{}\n    TaskTitles = @{}\n}\n",
    )?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(repo_root().join("scripts").join("sync-roadmap.ps1"))
        .env("REMOTTY_PLANNING_ROOT", &planning_root)
        .output()
        .context("failed to run sync-roadmap.ps1 without backlog")?;

    assert!(
        !output.status.success(),
        "sync-roadmap unexpectedly succeeded without backlog"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Backlog not found"),
        "stderr did not mention missing backlog: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn validate_planning_accepts_well_formed_inputs() -> Result<()> {
    let temp = TempDir::new()?;
    let backlog_path = temp.path().join("backlog.yaml");
    let title_path = temp.path().join("roadmap-title-ja.psd1");

    write_file(
        &backlog_path,
        "# === v0.1.1: Bootstrap patch ===\n- id: TASK-001\n    title: Create bridge foundation\n    status: done\n    priority: P0\n    target_version: v0.1.1\n    repo: remotty\n",
    )?;
    write_file(
        &title_path,
        "@{\n    VersionTitles = @{ \"v0.1.1\" = \"基盤\" }\n    TaskTitles = @{ \"TASK-001\" = \"ブリッジ基盤を作成\" }\n}\n",
    )?;

    let output = run_validate_planning(&backlog_path, &title_path)?;
    assert!(
        output.status.success(),
        "validate-planning failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn bootstrap_planning_generated_by_setup_is_valid() -> Result<()> {
    let temp = TempDir::new()?;
    let planning_root = temp.path().join("planning-root");
    let marker_path = temp.path().join("planning-root.txt");

    let setup = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(repo_root().join("scripts").join("setup-planning.ps1"))
        .arg("-PlanningRoot")
        .arg(&planning_root)
        .arg("-MarkerPath")
        .arg(&marker_path)
        .output()
        .context("failed to run setup-planning.ps1")?;

    assert!(
        setup.status.success(),
        "setup-planning failed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let output = run_validate_planning(
        &planning_root.join("backlog.yaml"),
        &planning_root.join("roadmap-title-ja.psd1"),
    )?;
    assert!(
        output.status.success(),
        "validate-planning failed for bootstrap files: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn validate_planning_allows_missing_title_map() -> Result<()> {
    let temp = TempDir::new()?;
    let backlog_path = temp.path().join("backlog.yaml");
    let missing_title_path = temp.path().join("roadmap-title-ja.psd1");

    write_file(
        &backlog_path,
        "# === v0.1.1: Bootstrap patch ===\n- id: TASK-001\n    title: Create bridge foundation\n    status: done\n    priority: P0\n    target_version: v0.1.1\n    repo: remotty\n",
    )?;

    let output = run_validate_planning(&backlog_path, &missing_title_path)?;
    assert!(
        output.status.success(),
        "validate-planning failed without title map: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn sync_roadmap_uses_explicit_env_overrides() -> Result<()> {
    let temp = TempDir::new()?;
    let planning_root = temp.path().join("planning-root");
    let explicit_backlog_path = temp.path().join("custom-backlog.yaml");
    let explicit_roadmap_path = temp.path().join("custom-ROADMAP.md");
    let explicit_title_path = temp.path().join("custom-roadmap-title-ja.psd1");
    fs::create_dir_all(&planning_root)?;

    write_file(
        &planning_root.join("backlog.yaml"),
        "# === v0.1.0: Wrong ===\n- id: TASK-999\n    title: Wrong planning root\n    status: done\n    priority: P0\n    target_version: v0.1.0\n    repo: remotty\n",
    )?;
    write_file(
        &explicit_backlog_path,
        "# === v0.1.1: Override ===\n- id: TASK-001\n    title: Use explicit override\n    status: done\n    priority: P0\n    target_version: v0.1.1\n    repo: remotty\n",
    )?;
    write_file(
        &explicit_title_path,
        "@{\n    VersionTitles = @{ \"v0.1.1\" = \"上書き\" }\n    TaskTitles = @{ \"TASK-001\" = \"個別上書きを使う\" }\n}\n",
    )?;

    let output = Command::new(powershell())
        .arg("-NoProfile")
        .arg("-File")
        .arg(repo_root().join("scripts").join("sync-roadmap.ps1"))
        .env("REMOTTY_PLANNING_ROOT", &planning_root)
        .env("REMOTTY_BACKLOG_PATH", &explicit_backlog_path)
        .env("REMOTTY_ROADMAP_PATH", &explicit_roadmap_path)
        .env("REMOTTY_ROADMAP_TITLE_JA_PATH", &explicit_title_path)
        .output()
        .context("failed to run sync-roadmap.ps1 with env override")?;

    assert!(
        output.status.success(),
        "sync-roadmap failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let roadmap = fs::read_to_string(&explicit_roadmap_path)?;
    assert!(roadmap.contains("### v0.1.1: 上書き"));
    assert!(roadmap.contains("個別上書きを使う"));
    assert!(!roadmap.contains("Wrong planning root"));

    Ok(())
}

#[test]
fn validate_planning_rejects_missing_required_backlog_fields() -> Result<()> {
    let temp = TempDir::new()?;
    let backlog_path = temp.path().join("backlog.yaml");
    let title_path = temp.path().join("roadmap-title-ja.psd1");

    write_file(
        &backlog_path,
        "# === v0.1.0: Bootstrap ===\n- id: TASK-001\n    title: Create bridge foundation\n    status: done\n    priority: P0\n    repo: remotty\n",
    )?;
    write_file(
        &title_path,
        "@{\n    VersionTitles = @{}\n    TaskTitles = @{}\n}\n",
    )?;

    let output = run_validate_planning(&backlog_path, &title_path)?;
    assert!(
        !output.status.success(),
        "validate-planning unexpectedly succeeded"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("target_version"),
        "stderr did not mention missing field: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn validate_planning_rejects_invalid_localization_file() -> Result<()> {
    let temp = TempDir::new()?;
    let backlog_path = temp.path().join("backlog.yaml");
    let title_path = temp.path().join("roadmap-title-ja.psd1");

    write_file(
        &backlog_path,
        "# === v0.1.0: Bootstrap ===\n- id: TASK-001\n    title: Create bridge foundation\n    status: done\n    priority: P0\n    target_version: v0.1.0\n    repo: remotty\n",
    )?;
    write_file(&title_path, "@{\nVersionTitles =\n")?;

    let output = run_validate_planning(&backlog_path, &title_path)?;
    assert!(
        !output.status.success(),
        "validate-planning unexpectedly succeeded"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("roadmap-title-ja.psd1"),
        "stderr did not mention localization file: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn validate_planning_rejects_invalid_backlog_values() -> Result<()> {
    let temp = TempDir::new()?;
    let backlog_path = temp.path().join("backlog.yaml");
    let title_path = temp.path().join("roadmap-title-ja.psd1");

    write_file(
        &backlog_path,
        "# === v0.1.0: Bootstrap ===\n- id: TASK-001\n    title: Create bridge foundation\n    status: progress\n    priority: HIGH\n    target_version: 1.0.0\n    repo: remotty\n\n- id: TASK-001\n    title: Add validation\n    status: active\n    priority: P1\n    target_version: v0.1.0\n    repo: remotty\n",
    )?;
    write_file(
        &title_path,
        "@{\n    VersionTitles = @{}\n    TaskTitles = @{}\n}\n",
    )?;

    let output = run_validate_planning(&backlog_path, &title_path)?;
    assert!(
        !output.status.success(),
        "validate-planning unexpectedly succeeded"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid status"),
        "stderr missing status: {stderr}"
    );
    assert!(
        stderr.contains("invalid priority"),
        "stderr missing priority: {stderr}"
    );
    assert!(
        stderr.contains("invalid target_version"),
        "stderr missing target_version: {stderr}"
    );
    assert!(
        stderr.contains("duplicated"),
        "stderr missing duplicate: {stderr}"
    );

    Ok(())
}
