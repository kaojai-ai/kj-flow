use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_major_capabilities() {
    let mut command = Command::cargo_bin("kj").unwrap();
    command
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("repo"))
        .stdout(predicate::str::contains("task"));
}

#[test]
fn doctor_is_machine_readable_without_configuration() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("missing.toml");
    let mut command = Command::cargo_bin("kj").unwrap();
    let output = command
        .env("KJ_CONFIG_PATH", config_path)
        .args(["--json", "doctor"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["healthy"], false);
    assert!(
        value["data"]["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "configuration")
    );
}

#[test]
fn parse_errors_are_json_when_requested() {
    let mut command = Command::cargo_bin("kj").unwrap();
    let output = command.args(["--json", "task", "show"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "command_failed");
}

#[test]
fn help_is_wrapped_when_json_is_requested() {
    let mut command = Command::cargo_bin("kj").unwrap();
    let output = command.args(["--json", "--help"]).output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    assert!(value["data"]["text"].as_str().unwrap().contains("Usage:"));
}

#[test]
fn json_rejects_streaming_foreground_commands() {
    let mut command = Command::cargo_bin("kj").unwrap();
    let output = command
        .args([
            "--json",
            "task",
            "start",
            "feature-auth",
            "frontend",
            "--foreground",
            "--",
            "/bin/true",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], false);
}

#[test]
fn init_accepts_a_custom_worktree_root() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let worktree_root = temp.path().join("checkouts");
    let config_path = temp.path().join("config.toml");
    std::fs::create_dir(&workspace).unwrap();

    let mut command = Command::cargo_bin("kj").unwrap();
    let output = command
        .env("KJ_CONFIG_PATH", config_path)
        .args([
            "--json",
            "init",
            "--workspace",
            workspace.to_str().unwrap(),
            "--worktree-root",
            worktree_root.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["data"]["worktree_root"],
        worktree_root.to_str().unwrap()
    );
    assert!(worktree_root.is_dir());
}
