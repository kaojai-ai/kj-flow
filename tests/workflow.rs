use kj_flow::app;
use kj_flow::model::UserConfig;
use serde_json::Value;
use serial_test::serial;
use std::fs;
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

struct TestWorkspace {
    root: TempDir,
    config_path: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("worktrees")).unwrap();
        fs::create_dir_all(root.path().join(".kj")).unwrap();
        fs::write(root.path().join(".kj/repos.toml"), "[repos]\n").unwrap();
        let config_path = root.path().join("config.toml");
        let config = UserConfig {
            workspace_root: root.path().to_path_buf(),
            worktree_root: None,
            port_start: 45_000,
            port_end: 45_100,
            branch_prefix: "codex".to_owned(),
        };
        fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();
        // SAFETY: integration tests are single-threaded per process invocation and restore no
        // shared application state; the environment variable is read only by this test process.
        unsafe { std::env::set_var("KJ_CONFIG_PATH", &config_path) };
        Self { root, config_path }
    }

    fn path(&self) -> &Path {
        self.root.path()
    }

    fn add_remote_repo(&self, name: &str) -> PathBuf {
        let bare = self.path().join(format!("{name}.git"));
        git(
            self.path(),
            [
                "init",
                "--bare",
                "--initial-branch=main",
                bare.to_str().unwrap(),
            ],
        );
        let repo = self.path().join(name);
        git(
            self.path(),
            ["clone", bare.to_str().unwrap(), repo.to_str().unwrap()],
        );
        git(&repo, ["config", "user.email", "test@example.com"]);
        git(&repo, ["config", "user.name", "Test"]);
        fs::write(repo.join("README.md"), format!("# {name}\n")).unwrap();
        git(&repo, ["add", "README.md"]);
        git(&repo, ["commit", "-m", "initial"]);
        git(&repo, ["push", "-u", "origin", "main"]);
        git(&repo, ["remote", "set-head", "origin", "main"]);
        repo
    }
}

#[test]
#[serial]
fn creates_tasks_in_a_configured_worktree_root() {
    let workspace = TestWorkspace::new();
    let configured_root = workspace.path().join("isolated-checkouts");
    fs::write(
        &workspace.config_path,
        toml::to_string(&UserConfig {
            workspace_root: workspace.path().to_path_buf(),
            worktree_root: Some(configured_root.clone()),
            port_start: 45_000,
            port_end: 45_100,
            branch_prefix: "agent".to_owned(),
        })
        .unwrap(),
    )
    .unwrap();
    workspace.add_remote_repo("catalog");

    app::task_create("custom-root", &["catalog".to_owned()]).unwrap();

    let worktree = configured_root.join("custom-root/catalog");
    assert!(worktree.join(".git").exists());
    assert_eq!(
        git_stdout(&worktree, ["branch", "--show-current"]),
        "agent/custom-root"
    );
}

#[test]
#[serial]
fn orphaned_invalid_task_state_does_not_block_new_task_creation() {
    let workspace = TestWorkspace::new();
    workspace.add_remote_repo("frontend");
    let state = workspace.path().join("worktrees/orphaned/.kj");
    fs::create_dir_all(&state).unwrap();
    fs::write(
        state.join("task.toml"),
        r#"
version = 2
id = "orphaned"
branch = "codex/orphaned"
created_at = 1

[[repositories]]
name = "frontend"
canonical_path = "/tmp/frontend"
worktree_path = "/tmp/missing-kj-flow-worktree"
base_ref = "origin/main"
base_sha = "unknown"
port = 45000

[[repositories]]
name = "contracts"
canonical_path = "/tmp/contracts"
worktree_path = "/tmp/missing-kj-flow-contracts"
base_ref = "origin/main"
base_sha = "unknown"
"#,
    )
    .unwrap();

    let listed = app::task_list().unwrap();
    assert_eq!(listed["tasks"].as_array().unwrap().len(), 0);
    assert_eq!(listed["invalid_tasks"].as_array().unwrap().len(), 1);

    app::task_create("new-task", &["frontend".to_owned()]).unwrap();
    assert!(
        workspace
            .path()
            .join("worktrees/new-task/frontend/.git")
            .exists()
    );
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = &self.config_path;
        unsafe { std::env::remove_var("KJ_CONFIG_PATH") };
    }
}

#[test]
#[serial]
fn creates_and_finishes_multi_repo_task() {
    let workspace = TestWorkspace::new();
    workspace.add_remote_repo("frontend");
    workspace.add_remote_repo("contracts");

    let created = app::task_create(
        "feature-auth",
        &["frontend".to_owned(), "contracts".to_owned()],
    )
    .unwrap();
    assert_eq!(created["id"], "feature-auth");
    assert_ne!(
        created["repositories"][0]["port"],
        created["repositories"][1]["port"]
    );
    assert!(
        workspace
            .path()
            .join("worktrees/feature-auth/frontend/.git")
            .exists()
    );
    assert_eq!(
        git_stdout(
            &workspace.path().join("worktrees/feature-auth/frontend"),
            ["branch", "--show-current"]
        ),
        "codex/feature-auth"
    );
    let frontend_worktree = workspace.path().join("worktrees/feature-auth/frontend");
    fs::write(frontend_worktree.join("shipped.txt"), "shipped\n").unwrap();
    git(
        &frontend_worktree,
        ["config", "user.email", "test@example.com"],
    );
    git(&frontend_worktree, ["config", "user.name", "Test"]);
    git(&frontend_worktree, ["add", "shipped.txt"]);
    git(&frontend_worktree, ["commit", "-m", "shipped"]);
    git(
        &frontend_worktree,
        ["push", "-u", "origin", "codex/feature-auth"],
    );

    let preview = app::task_finish("feature-auth", false).unwrap();
    assert_eq!(preview["applied"], false);
    app::task_finish("feature-auth", true).unwrap();
    assert!(!workspace.path().join("worktrees/feature-auth").exists());
    assert_eq!(
        git_stdout(
            &workspace.path().join("frontend"),
            ["branch", "--list", "codex/feature-auth"]
        ),
        "codex/feature-auth"
    );
}

#[test]
#[serial]
fn copies_only_safe_regular_environment_files() {
    let workspace = TestWorkspace::new();
    let repo = workspace.add_remote_repo("frontend");
    fs::write(repo.join(".env.local"), "SAFE_LOCAL=value\n").unwrap();
    fs::write(repo.join(".env.production"), "DO_NOT_COPY=value\n").unwrap();
    std::os::unix::fs::symlink("/tmp/outside", repo.join(".env")).unwrap();

    app::task_create("env-sync", &["frontend".to_owned()]).unwrap();
    let worktree = workspace.path().join("worktrees/env-sync/frontend");
    assert_eq!(
        fs::read_to_string(worktree.join(".env.local")).unwrap(),
        "SAFE_LOCAL=value\n"
    );
    assert_eq!(
        fs::metadata(worktree.join(".env.local"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert!(!worktree.join(".env.production").exists());
    assert!(!worktree.join(".env").exists());

    fs::write(repo.join(".env.local"), "SAFE_LOCAL=refreshed\n").unwrap();
    app::task_env_sync("env-sync", "frontend", true).unwrap();
    assert_eq!(
        fs::read_to_string(worktree.join(".env.local")).unwrap(),
        "SAFE_LOCAL=refreshed\n"
    );
}

#[test]
#[serial]
fn finish_refuses_dirty_or_unpushed_work() {
    let workspace = TestWorkspace::new();
    workspace.add_remote_repo("frontend");
    app::task_create("dirty-task", &["frontend".to_owned()]).unwrap();
    let worktree = workspace.path().join("worktrees/dirty-task/frontend");
    fs::write(worktree.join("dirty.txt"), "dirty\n").unwrap();
    assert!(app::task_finish("dirty-task", true).is_err());

    git(&worktree, ["add", "dirty.txt"]);
    git(&worktree, ["config", "user.email", "test@example.com"]);
    git(&worktree, ["config", "user.name", "Test"]);
    git(&worktree, ["commit", "-m", "unpushed"]);
    assert!(app::task_finish("dirty-task", true).is_err());
}

#[test]
#[serial]
fn starts_logs_and_stops_background_process() {
    let workspace = TestWorkspace::new();
    workspace.add_remote_repo("service");
    app::task_create("runtime-task", &["service".to_owned()]).unwrap();
    let state = workspace.path().join("worktrees/runtime-task/.kj");
    assert_eq!(file_mode(&state), 0o700);
    assert_eq!(file_mode(&state.join("task.toml")), 0o600);

    let started = app::task_start(
        "runtime-task",
        "service",
        false,
        vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            "echo ready; sleep 30".to_owned(),
        ],
    )
    .unwrap();
    assert!(started["pid"].as_u64().unwrap() > 0);
    assert!(
        workspace
            .path()
            .join("worktrees/runtime-task/.kj/pids/service.pid")
            .is_file()
    );
    assert!(
        workspace
            .path()
            .join("worktrees/runtime-task/.kj/runtime/service.toml")
            .is_file()
    );
    assert_eq!(file_mode(&state.join("logs/service.log")), 0o600);
    assert!(
        !fs::read_to_string(state.join("task.toml"))
            .unwrap()
            .contains("echo ready")
    );
    thread::sleep(Duration::from_millis(250));
    let logs = app::task_logs("runtime-task", "service", false, true).unwrap();
    assert!(logs["content"].as_str().unwrap().contains("ready"));
    let stopped = app::task_stop("runtime-task", Some("service")).unwrap();
    assert_eq!(array_len(&stopped["stopped"]), 1);
    assert!(
        !workspace
            .path()
            .join("worktrees/runtime-task/.kj/pids/service.pid")
            .exists()
    );
}

#[test]
#[serial]
fn start_refuses_an_occupied_reserved_port() {
    let workspace = TestWorkspace::new();
    workspace.add_remote_repo("service");
    app::task_create("port-check", &["service".to_owned()]).unwrap();
    let task = app::task_show("port-check").unwrap();
    let port = task["repositories"][0]["port"].as_u64().unwrap() as u16;
    let _listener = TcpListener::bind(("127.0.0.1", port)).unwrap();

    let error =
        app::task_start("port-check", "service", false, vec!["/bin/true".to_owned()]).unwrap_err();
    assert!(format!("{error:#}").contains("reserved port"));
}

#[test]
#[serial]
fn rolls_back_created_worktrees_when_later_add_fails() {
    let workspace = TestWorkspace::new();
    workspace.add_remote_repo("alpha");
    let beta = workspace.add_remote_repo("beta");
    let hook = beta.join(".git/hooks/post-checkout");
    fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();

    let result = app::task_create("rollback-task", &["alpha".to_owned(), "beta".to_owned()]);
    assert!(result.is_err());
    assert!(!workspace.path().join("worktrees/rollback-task").exists());
    assert!(
        git_stdout(
            &workspace.path().join("alpha"),
            ["branch", "--list", "codex/rollback-task"]
        )
        .is_empty()
    );
}

fn array_len(value: &Value) -> usize {
    value.as_array().unwrap().len()
}

fn file_mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

fn git<const N: usize>(cwd: &Path, args: [&str; N]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_stdout<const N: usize>(cwd: &Path, args: [&str; N]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
