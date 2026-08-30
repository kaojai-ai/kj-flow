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
        .stdout(predicate::str::contains("env"))
        .stdout(predicate::str::contains("repo"))
        .stdout(predicate::str::contains("task"));
}

#[test]
fn task_create_help_lists_environment_file_selection() {
    let mut command = Command::cargo_bin("kj").unwrap();
    command
        .args(["task", "create", "feature-auth", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--env-file"));
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
fn env_loads_current_directory_file_for_command() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join(".env.local"),
        "KJ_ENV_TEST=\"loaded value\"\n",
    )
    .unwrap();
    let mut command = Command::cargo_bin("kj").unwrap();
    command
        .current_dir(temp.path())
        .args([
            "env",
            "local",
            "--",
            "/bin/sh",
            "-c",
            "printf '%s' \"$KJ_ENV_TEST\"",
        ])
        .assert()
        .success()
        .stdout("loaded value")
        .stderr(predicate::str::contains("Loaded .env.local"));
}

#[test]
fn env_refuses_symlinked_file() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.env");
    std::fs::write(&source, "KJ_ENV_TEST=value\n").unwrap();
    std::os::unix::fs::symlink(source, temp.path().join(".env.local")).unwrap();

    let mut command = Command::cargo_bin("kj").unwrap();
    command
        .current_dir(temp.path())
        .args(["env", "local", "--", "/bin/true"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "environment file must be a regular file",
        ));
}
