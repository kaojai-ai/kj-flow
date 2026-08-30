use crate::config::{
    config_path, load_runtime_overrides, load_user_config, save_user_config, validate_config,
};
use crate::git::{
    add_worktree, commits_are_pushed, delete_local_branch, discover_repositories,
    ensure_branch_available, path_is_tracked, prepare_base, remove_worktree, worktree_clean,
};
use crate::model::{
    DEFAULT_ENV_FILES, DoctorCheck, ProcessRecord, RepoRuntime, RepositoryInfo, RuntimeOverrides,
    TASK_SCHEMA_VERSION, TaskManifest, TaskRepository, UserConfig, default_environment_files,
};
use anyhow::{Context, Result, anyhow, bail};
use fs2::FileExt;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::TcpListener;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn env_load(environment: &str, command: Vec<String>) -> Result<Value> {
    if environment.is_empty()
        || !environment
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        bail!("environment must contain only letters, numbers, hyphens, or underscores");
    }

    let current_dir = env::current_dir().context("resolve current directory")?;
    let file_name = if environment == "default" {
        ".env".to_owned()
    } else {
        format!(".env.{environment}")
    };
    let env_path = current_dir.join(&file_name);
    if !regular_file(&env_path) {
        bail!(
            "environment file must be a regular file: {}",
            env_path.display()
        );
    }

    let variables = dotenvy::from_path_iter(&env_path)
        .with_context(|| format!("parse {}", env_path.display()))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("parse {}", env_path.display()))?;

    eprintln!("Loaded {file_name}");

    let error = if command.is_empty() {
        let shell = env::var_os("SHELL")
            .filter(|value| Path::new(value).is_absolute())
            .unwrap_or_else(|| "/bin/sh".into());
        Command::new(shell)
            .arg("-i")
            .current_dir(current_dir)
            .envs(variables)
            .exec()
    } else {
        Command::new(&command[0])
            .args(&command[1..])
            .current_dir(current_dir)
            .envs(variables)
            .exec()
    };

    Err(error).context("launch environment command")
}

pub fn init(workspace: PathBuf) -> Result<Value> {
    let config = validate_config(UserConfig::new(workspace))?;
    fs::create_dir_all(config.worktree_root())
        .with_context(|| format!("create worktree root {}", config.worktree_root().display()))?;
    let path = save_user_config(&config)?;
    Ok(json!({
        "workspace_root": config.workspace_root,
        "worktree_root": config.worktree_root(),
        "config_path": path
    }))
}

pub fn doctor() -> Value {
    let mut checks = Vec::new();
    checks.push(command_check("git"));
    checks.push(optional_command_check(
        "codex",
        "optional; required only for `kj task codex`",
    ));

    let config_result = load_user_config();
    match config_result {
        Ok(config) => {
            checks.push(DoctorCheck {
                name: "configuration".to_owned(),
                ok: true,
                detail: config_path()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|error| error.to_string()),
            });
            checks.push(DoctorCheck {
                name: "workspace_non_git".to_owned(),
                ok: !config.workspace_root.join(".git").exists(),
                detail: config.workspace_root.display().to_string(),
            });
            let overrides = load_runtime_overrides(&config.workspace_root);
            checks.push(DoctorCheck {
                name: "runtime_overrides".to_owned(),
                ok: overrides.is_ok(),
                detail: overrides
                    .as_ref()
                    .map(|value| format!("{} configured repositories", value.repos.len()))
                    .unwrap_or_else(|error| error.to_string()),
            });
            let overrides = overrides.unwrap_or_default();
            match discover_repositories(&config.workspace_root, &overrides) {
                Ok(repositories) => {
                    checks.push(DoctorCheck {
                        name: "repositories".to_owned(),
                        ok: !repositories.is_empty(),
                        detail: format!("{} discovered", repositories.len()),
                    });
                    let env_count = repositories
                        .iter()
                        .flat_map(|repo| {
                            DEFAULT_ENV_FILES
                                .iter()
                                .map(move |name| repo.path.join(name))
                        })
                        .filter(|path| regular_file(path))
                        .count();
                    checks.push(DoctorCheck {
                        name: "environment_sources".to_owned(),
                        ok: true,
                        detail: format!("{env_count} safe local files available"),
                    });
                }
                Err(error) => checks.push(DoctorCheck {
                    name: "repositories".to_owned(),
                    ok: false,
                    detail: error.to_string(),
                }),
            }
            let worktree_root = config.worktree_root();
            checks.push(DoctorCheck {
                name: "worktree_root".to_owned(),
                ok: worktree_root.is_dir()
                    && fs::metadata(&worktree_root)
                        .is_ok_and(|metadata| !metadata.permissions().readonly()),
                detail: worktree_root.display().to_string(),
            });
            match list_task_manifests(&config) {
                Ok(tasks) => {
                    let stale = tasks
                        .iter()
                        .flat_map(|task| task.processes.values())
                        .filter(|process| !managed_process_alive(process.pid))
                        .count();
                    let pid_state = pid_state_error_count(&config, &tasks);
                    let pid_state_errors = pid_state.as_ref().copied().unwrap_or(1);
                    checks.push(DoctorCheck {
                        name: "task_state".to_owned(),
                        ok: stale == 0 && matches!(pid_state, Ok(0)),
                        detail: format!(
                            "{} tasks, {stale} stale process records, {pid_state_errors} PID state errors",
                            tasks.len()
                        ),
                    });

                    let leases = tasks
                        .iter()
                        .flat_map(|task| task.repositories.iter().map(|repo| repo.port))
                        .collect::<Vec<_>>();
                    let unique = leases.iter().copied().collect::<BTreeSet<_>>();
                    let out_of_range = leases
                        .iter()
                        .filter(|port| **port < config.port_start || **port > config.port_end)
                        .count();
                    checks.push(DoctorCheck {
                        name: "port_registry".to_owned(),
                        ok: unique.len() == leases.len() && out_of_range == 0,
                        detail: format!(
                            "{} leases, {} duplicates, {out_of_range} out of range",
                            leases.len(),
                            leases.len() - unique.len()
                        ),
                    });
                }
                Err(error) => checks.push(DoctorCheck {
                    name: "task_state".to_owned(),
                    ok: false,
                    detail: error.to_string(),
                }),
            }
            checks.push(DoctorCheck {
                name: "port_range".to_owned(),
                ok: config.port_start <= config.port_end,
                detail: format!("{}-{}", config.port_start, config.port_end),
            });
        }
        Err(error) => checks.push(DoctorCheck {
            name: "configuration".to_owned(),
            ok: false,
            detail: error.to_string(),
        }),
    }

    let healthy = checks.iter().all(|check| check.ok);
    json!({
        "healthy": healthy,
        "auth_required": false,
        "checks": checks
    })
}

pub fn repo_list() -> Result<Value> {
    let config = load_user_config()?;
    let overrides = load_runtime_overrides(&config.workspace_root)?;
    let repositories = discover_repositories(&config.workspace_root, &overrides)?;
    Ok(json!({ "repositories": repositories }))
}

pub fn task_create(task_id: &str, requested_repos: &[String]) -> Result<Value> {
    task_create_with_env_files(task_id, requested_repos, &[])
}

pub fn task_create_with_env_files(
    task_id: &str,
    requested_repos: &[String],
    requested_env_files: &[String],
) -> Result<Value> {
    validate_task_id(task_id)?;
    let environment_files = resolve_environment_files(requested_env_files)?;
    if requested_repos.is_empty() {
        bail!("at least one --repo is required");
    }
    let unique = requested_repos.iter().collect::<BTreeSet<_>>();
    if unique.len() != requested_repos.len() {
        bail!("repository names must be unique");
    }

    let config = load_user_config()?;
    let _workspace_lock = lock_workspace(&config)?;
    let task_root = task_root(&config, task_id);
    if task_root.exists() {
        bail!("task path already exists: {}", task_root.display());
    }
    let overrides = load_runtime_overrides(&config.workspace_root)?;
    let discovered = discover_repositories(&config.workspace_root, &overrides)?;
    let by_name = discovered
        .into_iter()
        .map(|repo| (repo.name.clone(), repo))
        .collect::<BTreeMap<_, _>>();
    let branch = format!("{}/{task_id}", config.branch_prefix);
    let mut prepared = Vec::new();

    for name in requested_repos {
        let repo = by_name
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("unknown repository: {name}"))?;
        ensure_branch_available(&repo, &branch)?;
        let worktree_path = task_root.join(name);
        if worktree_path.exists() {
            bail!("worktree path already exists: {}", worktree_path.display());
        }
        let (base_ref, base_sha) = prepare_base(&repo)?;
        prepared.push((repo, worktree_path, base_ref, base_sha));
    }

    let ports = allocate_ports(&config, prepared.len())?;
    create_private_dir_all(&task_state_dir(&task_root))
        .with_context(|| format!("create task state for {task_id}"))?;

    let mut created: Vec<(RepositoryInfo, PathBuf)> = Vec::new();
    let create_result: Result<Vec<TaskRepository>> = prepared
        .into_iter()
        .zip(ports)
        .map(|((repo, worktree_path, base_ref, base_sha), port)| {
            if let Err(error) = add_worktree(&repo, &worktree_path, &branch, &base_ref) {
                if worktree_path.exists() {
                    let _ = remove_worktree(&repo.path, &worktree_path, true);
                }
                let _ = delete_local_branch(&repo.path, &branch);
                return Err(error);
            }
            created.push((repo.clone(), worktree_path.clone()));
            copy_env_files(&repo.path, &worktree_path, false, &environment_files)?;
            Ok(TaskRepository {
                name: repo.name,
                canonical_path: repo.path,
                worktree_path,
                base_ref,
                base_sha,
                port,
            })
        })
        .collect();

    let repositories = match create_result {
        Ok(repositories) => repositories,
        Err(error) => {
            rollback_created_worktrees(&created, &branch);
            let _ = fs::remove_dir_all(&task_root);
            return Err(error.context("task creation rolled back"));
        }
    };

    let manifest = TaskManifest {
        version: TASK_SCHEMA_VERSION,
        id: task_id.to_owned(),
        branch,
        created_at: now_epoch(),
        environment_files,
        repositories,
        processes: BTreeMap::new(),
    };
    if let Err(error) = save_task_manifest(&task_root, &manifest)
        .and_then(|_| save_runtime_environment(&task_root, &manifest))
    {
        rollback_created_worktrees(&created, &manifest.branch);
        let _ = fs::remove_dir_all(&task_root);
        return Err(error.context("task creation rolled back"));
    }
    Ok(task_summary(&manifest))
}

pub fn task_list() -> Result<Value> {
    let config = load_user_config()?;
    let tasks = list_task_manifests(&config)?;
    let summaries = tasks.iter().map(task_summary).collect::<Vec<_>>();
    Ok(json!({ "tasks": summaries }))
}

pub fn task_show(task_id: &str) -> Result<Value> {
    validate_task_id(task_id)?;
    let config = load_user_config()?;
    let manifest = load_task_manifest(&task_root(&config, task_id))?;
    Ok(task_summary(&manifest))
}

pub fn task_env_sync(task_id: &str, repo_name: &str, apply: bool) -> Result<Value> {
    validate_task_id(task_id)?;
    let config = load_user_config()?;
    let task_root = task_root(&config, task_id);
    let _lock = lock_task(&task_root)?;
    let manifest = load_task_manifest(&task_root)?;
    let repo = find_task_repo(&manifest, repo_name)?;
    let environment_files = resolve_environment_files(&manifest.environment_files)?;
    let files = if apply {
        copy_env_files(
            &repo.canonical_path,
            &repo.worktree_path,
            true,
            &environment_files,
        )?
    } else {
        preview_env_sync(
            &repo.canonical_path,
            &repo.worktree_path,
            &environment_files,
        )
    };
    Ok(json!({
        "task_id": task_id,
        "repository": repo_name,
        "applied": apply,
        "files": files
    }))
}

pub fn task_start(
    task_id: &str,
    repo_name: &str,
    foreground: bool,
    provided_command: Vec<String>,
) -> Result<Value> {
    validate_task_id(task_id)?;
    let config = load_user_config()?;
    let task_root = task_root(&config, task_id);
    let _lock = lock_task(&task_root)?;
    let mut manifest = load_task_manifest(&task_root)?;
    let repo = find_task_repo(&manifest, repo_name)?.clone();

    let mut cleared_stale_state = false;
    if let Some(process) = manifest.processes.get(repo_name) {
        if managed_process_alive(process.pid) {
            bail!("{repo_name} is already running with PID {}", process.pid);
        }
        manifest.processes.remove(repo_name);
        let _ = fs::remove_file(pid_path(&task_root, repo_name));
        cleared_stale_state = true;
    } else {
        let existing_pid_path = pid_path(&task_root, repo_name);
        if existing_pid_path.exists() {
            let pid = read_pid_file(&existing_pid_path)?;
            if managed_process_alive(pid) {
                bail!(
                    "PID file records a running process for {repo_name} ({pid}); run doctor and stop it"
                );
            }
            fs::remove_file(&existing_pid_path)
                .with_context(|| format!("remove stale {}", existing_pid_path.display()))?;
        }
    }
    if cleared_stale_state {
        save_task_manifest(&task_root, &manifest)?;
    }
    ensure_port_available(repo.port)
        .with_context(|| format!("reserved port {} is occupied", repo.port))?;

    let overrides = load_runtime_overrides(&config.workspace_root)?;
    let command = resolve_dev_command(&repo, &overrides, provided_command)?;
    if foreground {
        let status = configured_command(&command, &repo, task_id)
            .status()
            .with_context(|| format!("start {repo_name} in foreground"))?;
        ensure_process_success(status, &command)?;
        return Ok(json!({
            "task_id": task_id,
            "repository": repo_name,
            "foreground": true,
            "exit_status": status.code()
        }));
    }

    let log_path = task_state_dir(&task_root)
        .join("logs")
        .join(format!("{repo_name}.log"));
    if let Some(parent) = log_path.parent() {
        create_private_dir_all(parent)
            .with_context(|| format!("create log directory {}", parent.display()))?;
    }
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&log_path)
        .with_context(|| format!("open {}", log_path.display()))?;
    stdout
        .set_permissions(fs::Permissions::from_mode(0o600))
        .with_context(|| format!("secure {}", log_path.display()))?;
    let stderr = stdout.try_clone().context("clone log file handle")?;
    let mut child_command = configured_command(&command, &repo, task_id);
    child_command
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .process_group(0);
    let mut child = child_command
        .spawn()
        .with_context(|| format!("start {repo_name}"))?;
    let pid = child.id();
    thread::spawn(move || {
        let _ = child.wait();
    });
    if let Err(error) = write_pid_file(&task_root, repo_name, pid) {
        let _ = stop_process_group(pid);
        return Err(error);
    }
    let record = ProcessRecord {
        pid,
        started_at: now_epoch(),
        log_path: log_path.clone(),
    };
    manifest.processes.insert(repo_name.to_owned(), record);
    if let Err(error) = save_task_manifest(&task_root, &manifest) {
        let _ = fs::remove_file(pid_path(&task_root, repo_name));
        let _ = stop_process_group(pid);
        return Err(error);
    }
    Ok(json!({
        "task_id": task_id,
        "repository": repo_name,
        "foreground": false,
        "pid": pid,
        "port": repo.port,
        "log_path": log_path,
        "command": command
    }))
}

pub fn task_stop(task_id: &str, repo_name: Option<&str>) -> Result<Value> {
    validate_task_id(task_id)?;
    let config = load_user_config()?;
    let task_root = task_root(&config, task_id);
    let _lock = lock_task(&task_root)?;
    let mut manifest = load_task_manifest(&task_root)?;
    let targets = if let Some(name) = repo_name {
        if !manifest.processes.contains_key(name) {
            bail!("no recorded process for {name}");
        }
        vec![name.to_owned()]
    } else {
        manifest.processes.keys().cloned().collect()
    };
    let mut stopped = Vec::new();
    for name in targets {
        if let Some(process) = manifest.processes.remove(&name) {
            stop_process_group(process.pid)?;
            stopped.push(json!({
                "repository": name,
                "pid": process.pid
            }));
        }
    }
    save_task_manifest(&task_root, &manifest)?;
    for item in &stopped {
        if let Some(name) = item.get("repository").and_then(Value::as_str) {
            let path = pid_path(&task_root, name);
            if path.exists() {
                fs::remove_file(&path)
                    .with_context(|| format!("remove PID file {}", path.display()))?;
            }
        }
    }
    Ok(json!({ "task_id": task_id, "stopped": stopped }))
}

pub fn task_logs(task_id: &str, repo_name: &str, follow: bool, json_mode: bool) -> Result<Value> {
    validate_task_id(task_id)?;
    if follow && json_mode {
        bail!("--json cannot be combined with --follow");
    }
    let config = load_user_config()?;
    let root = task_root(&config, task_id);
    let manifest = load_task_manifest(&root)?;
    find_task_repo(&manifest, repo_name)?;
    let path = root
        .join(".kj")
        .join("logs")
        .join(format!("{repo_name}.log"));
    if !path.is_file() {
        bail!("log does not exist: {}", path.display());
    }
    if follow {
        let status = Command::new("tail")
            .args(["-n", "200", "-f"])
            .arg(&path)
            .status()
            .with_context(|| format!("follow {}", path.display()))?;
        if !status.success() {
            bail!("tail exited with {status}");
        }
        return Ok(json!({ "log_path": path, "followed": true }));
    }
    let content = tail_file(&path, 200)?;
    Ok(json!({
        "task_id": task_id,
        "repository": repo_name,
        "log_path": path,
        "content": content
    }))
}

pub fn task_finish(task_id: &str, apply: bool) -> Result<Value> {
    validate_task_id(task_id)?;
    let config = load_user_config()?;
    let root = task_root(&config, task_id);
    let _lock = lock_task(&root)?;
    let manifest = load_task_manifest(&root)?;
    if !manifest.processes.is_empty() {
        bail!("task still has recorded processes; run `kj task stop {task_id}`");
    }
    let remaining_pid_files = pid_files(&root)?;
    if !remaining_pid_files.is_empty() {
        bail!("task still has PID files; run `kj --json doctor` before finishing");
    }
    let mut actions = Vec::new();
    for repo in &manifest.repositories {
        if !worktree_clean(&repo.worktree_path)? {
            bail!("worktree is dirty: {}", repo.worktree_path.display());
        }
        if !commits_are_pushed(&repo.worktree_path, &manifest.branch, &repo.base_sha)? {
            bail!("task commits are not pushed for repository {}", repo.name);
        }
        actions.push(json!({
            "repository": repo.name,
            "remove_worktree": repo.worktree_path,
            "retain_branch": manifest.branch
        }));
    }
    if apply {
        for repo in &manifest.repositories {
            remove_worktree(&repo.canonical_path, &repo.worktree_path, false)?;
        }
        fs::remove_dir_all(&root)
            .with_context(|| format!("remove task state {}", root.display()))?;
    }
    Ok(json!({
        "task_id": task_id,
        "applied": apply,
        "actions": actions
    }))
}

pub fn codex_arguments(
    manifest: &TaskManifest,
    primary: Option<&str>,
    extra: &[String],
) -> Result<Vec<String>> {
    let primary_name = match primary {
        Some(name) => name,
        None if manifest.repositories.len() == 1 => &manifest.repositories[0].name,
        None => bail!("--primary is required for a multi-repository task"),
    };
    let primary_repo = find_task_repo(manifest, primary_name)?;
    let mut args = vec![
        "-C".to_owned(),
        primary_repo.worktree_path.display().to_string(),
        "--sandbox".to_owned(),
        "workspace-write".to_owned(),
    ];
    let mut additional = manifest
        .repositories
        .iter()
        .filter(|repo| repo.name != primary_name)
        .collect::<Vec<_>>();
    additional.sort_by(|a, b| a.name.cmp(&b.name));
    for repo in additional {
        args.push("--add-dir".to_owned());
        args.push(repo.worktree_path.display().to_string());
    }
    args.extend_from_slice(extra);
    Ok(args)
}

pub fn task_codex(task_id: &str, primary: Option<&str>, extra: Vec<String>) -> Result<Value> {
    validate_task_id(task_id)?;
    let config = load_user_config()?;
    let root = task_root(&config, task_id);
    let manifest = load_task_manifest(&root)?;
    let args = codex_arguments(&manifest, primary, &extra)?;
    let error = Command::new("codex").args(&args).exec();
    Err(anyhow!("launch codex: {error}"))
}

fn command_check(name: &str) -> DoctorCheck {
    let found = env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| {
            fs::metadata(directory.join(name)).is_ok_and(|metadata| {
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            })
        })
    });
    DoctorCheck {
        name: format!("command_{name}"),
        ok: found,
        detail: if found {
            "available".to_owned()
        } else {
            "missing from PATH".to_owned()
        },
    }
}

fn optional_command_check(name: &str, missing_detail: &str) -> DoctorCheck {
    let mut check = command_check(name);
    if !check.ok {
        check.ok = true;
        check.detail = missing_detail.to_owned();
    }
    check
}

fn task_root(config: &UserConfig, task_id: &str) -> PathBuf {
    config.worktree_root().join(task_id)
}

fn task_state_dir(task_root: &Path) -> PathBuf {
    task_root.join(".kj")
}

fn task_manifest_path(task_root: &Path) -> PathBuf {
    task_state_dir(task_root).join("task.toml")
}

fn runtime_path(task_root: &Path, repo_name: &str) -> PathBuf {
    task_state_dir(task_root)
        .join("runtime")
        .join(format!("{repo_name}.toml"))
}

fn pid_path(task_root: &Path, repo_name: &str) -> PathBuf {
    task_state_dir(task_root)
        .join("pids")
        .join(format!("{repo_name}.pid"))
}

fn load_task_manifest(task_root: &Path) -> Result<TaskManifest> {
    let path = task_manifest_path(task_root);
    let contents = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let manifest: TaskManifest =
        toml::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
    if manifest.version != TASK_SCHEMA_VERSION {
        bail!(
            "unsupported task schema {} in {}",
            manifest.version,
            path.display()
        );
    }
    Ok(manifest)
}

fn save_task_manifest(task_root: &Path, manifest: &TaskManifest) -> Result<()> {
    let state = task_state_dir(task_root);
    create_private_dir_all(&state)
        .with_context(|| format!("create task state {}", state.display()))?;
    let path = task_manifest_path(task_root);
    let temp_path = state.join("task.toml.tmp");
    let contents = toml::to_string_pretty(manifest).context("serialize task manifest")?;
    secure_write(&temp_path, contents.as_bytes())?;
    fs::rename(&temp_path, &path).with_context(|| format!("replace {}", path.display()))?;
    Ok(())
}

fn save_runtime_environment(task_root: &Path, manifest: &TaskManifest) -> Result<()> {
    for repo in &manifest.repositories {
        let path = runtime_path(task_root, &repo.name);
        let parent = path.parent().context("runtime path has no parent")?;
        create_private_dir_all(parent)
            .with_context(|| format!("create runtime directory {}", parent.display()))?;
        let contents = toml::to_string_pretty(&BTreeMap::from([
            ("PORT", repo.port.to_string()),
            ("KJ_TASK_ID", manifest.id.clone()),
            ("KJ_REPO", repo.name.clone()),
        ]))
        .context("serialize runtime environment")?;
        secure_write(&path, contents.as_bytes())?;
    }
    Ok(())
}

fn write_pid_file(task_root: &Path, repo_name: &str, pid: u32) -> Result<PathBuf> {
    let path = pid_path(task_root, repo_name);
    let parent = path.parent().context("PID path has no parent")?;
    create_private_dir_all(parent)
        .with_context(|| format!("create PID directory {}", parent.display()))?;
    secure_write(&path, format!("{pid}\n").as_bytes())?;
    Ok(path)
}

fn read_pid_file(path: &Path) -> Result<u32> {
    let contents = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    contents
        .trim()
        .parse::<u32>()
        .with_context(|| format!("parse PID from {}", path.display()))
}

fn pid_files(task_root: &Path) -> Result<Vec<(String, PathBuf)>> {
    let directory = task_state_dir(task_root).join("pids");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(&directory)
        .with_context(|| format!("read PID directory {}", directory.display()))?
    {
        let entry = entry.context("read PID directory entry")?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("pid") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .context("PID filename is not UTF-8")?
            .to_owned();
        files.push((name, path));
    }
    Ok(files)
}

fn pid_state_error_count(config: &UserConfig, tasks: &[TaskManifest]) -> Result<usize> {
    let mut errors = 0;
    for task in tasks {
        let root = config.worktree_root().join(&task.id);
        let files = pid_files(&root)?;
        for (repo_name, process) in &task.processes {
            let path = pid_path(&root, repo_name);
            if !matches!(read_pid_file(&path), Ok(pid) if pid == process.pid) {
                errors += 1;
            }
        }
        errors += files
            .iter()
            .filter(|(repo_name, _)| !task.processes.contains_key(repo_name))
            .count();
    }
    Ok(errors)
}

fn list_task_manifests(config: &UserConfig) -> Result<Vec<TaskManifest>> {
    let root = config.worktree_root();
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut tasks = Vec::new();
    for entry in fs::read_dir(&root).with_context(|| format!("read {}", root.display()))? {
        let entry = entry.context("read task directory entry")?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = task_manifest_path(&path);
        if manifest_path.is_file() {
            tasks.push(load_task_manifest(&path)?);
        }
    }
    tasks.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(tasks)
}

fn task_summary(manifest: &TaskManifest) -> Value {
    let process_status = manifest
        .processes
        .iter()
        .map(|(name, process)| {
            (
                name.clone(),
                json!({
                    "pid": process.pid,
                    "alive": managed_process_alive(process.pid),
                    "log_path": process.log_path
                }),
            )
        })
        .collect::<BTreeMap<_, _>>();
    json!({
        "id": manifest.id,
        "branch": manifest.branch,
        "created_at": manifest.created_at,
        "environment_files": manifest.environment_files,
        "repositories": manifest.repositories,
        "processes": process_status
    })
}

fn find_task_repo<'a>(manifest: &'a TaskManifest, repo_name: &str) -> Result<&'a TaskRepository> {
    manifest
        .repositories
        .iter()
        .find(|repo| repo.name == repo_name)
        .ok_or_else(|| anyhow!("task {} does not include {repo_name}", manifest.id))
}

fn lock_task(task_root: &Path) -> Result<File> {
    if !task_manifest_path(task_root).is_file() {
        bail!("task does not exist: {}", task_root.display());
    }
    let path = task_state_dir(task_root).join("task.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(&path)
        .with_context(|| format!("open task lock {}", path.display()))?;
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .with_context(|| format!("secure {}", path.display()))?;
    FileExt::lock_exclusive(&file).with_context(|| format!("lock task {}", task_root.display()))?;
    Ok(file)
}

fn lock_workspace(config: &UserConfig) -> Result<File> {
    fs::create_dir_all(config.worktree_root())
        .with_context(|| format!("create {}", config.worktree_root().display()))?;
    let path = config.worktree_root().join(".kj.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .with_context(|| format!("secure {}", path.display()))?;
    FileExt::lock_exclusive(&file).context("lock workspace")?;
    Ok(file)
}

fn allocate_ports(config: &UserConfig, count: usize) -> Result<Vec<u16>> {
    let used = list_task_manifests(config)?
        .into_iter()
        .flat_map(|task| task.repositories.into_iter().map(|repo| repo.port))
        .collect::<BTreeSet<_>>();
    let mut allocated = Vec::new();
    for port in config.port_start..=config.port_end {
        if used.contains(&port) || allocated.contains(&port) {
            continue;
        }
        if ensure_port_available(port).is_ok() {
            allocated.push(port);
            if allocated.len() == count {
                return Ok(allocated);
            }
        }
    }
    bail!(
        "cannot allocate {count} ports in range {}-{}",
        config.port_start,
        config.port_end
    )
}

fn ensure_port_available(port: u16) -> Result<()> {
    let loopback =
        TcpListener::bind(("127.0.0.1", port)).with_context(|| format!("bind 127.0.0.1:{port}"))?;
    drop(loopback);
    TcpListener::bind(("0.0.0.0", port))
        .with_context(|| format!("bind 0.0.0.0:{port}"))
        .map(drop)
}

fn rollback_created_worktrees(created: &[(RepositoryInfo, PathBuf)], branch: &str) {
    for (repo, path) in created.iter().rev() {
        let _ = remove_worktree(&repo.path, path, true);
        let _ = delete_local_branch(&repo.path, branch);
    }
}

fn copy_env_files(
    source: &Path,
    destination: &Path,
    overwrite: bool,
    environment_files: &[String],
) -> Result<Vec<Value>> {
    let mut results = Vec::new();
    for name in environment_files {
        let source_path = source.join(name);
        let target_path = destination.join(name);
        if !regular_file(&source_path) {
            results.push(json!({ "file": name, "status": "source_missing_or_unsafe" }));
            continue;
        }
        if path_is_tracked(destination, name)? {
            results.push(json!({ "file": name, "status": "kept_tracked" }));
            continue;
        }
        if let Ok(metadata) = fs::symlink_metadata(&target_path) {
            if !metadata.file_type().is_file() {
                results.push(json!({ "file": name, "status": "kept_unsafe_destination" }));
                continue;
            }
            if !overwrite {
                results.push(json!({ "file": name, "status": "kept_existing" }));
                continue;
            }
        }
        fs::copy(&source_path, &target_path).with_context(|| format!("copy local {name}"))?;
        fs::set_permissions(&target_path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("secure local {name}"))?;
        results.push(json!({ "file": name, "status": "copied" }));
    }
    Ok(results)
}

fn preview_env_sync(source: &Path, destination: &Path, environment_files: &[String]) -> Vec<Value> {
    environment_files
        .iter()
        .map(|name| {
            let source_safe = regular_file(&source.join(name));
            let target_exists = destination.join(name).exists();
            json!({
                "file": name,
                "source_safe": source_safe,
                "target_exists": target_exists,
                "action": if source_safe { "copy_on_apply" } else { "skip" }
            })
        })
        .collect()
}

fn resolve_environment_files(requested: &[String]) -> Result<Vec<String>> {
    let names = if requested.is_empty() {
        default_environment_files()
    } else {
        requested.to_vec()
    };
    let mut seen = BTreeSet::new();
    let mut resolved = Vec::new();
    for name in names {
        validate_environment_file_name(&name)?;
        if seen.insert(name.clone()) {
            resolved.push(name);
        }
    }
    Ok(resolved)
}

fn validate_environment_file_name(name: &str) -> Result<()> {
    let valid = if name == ".env" {
        true
    } else if let Some(suffix) = name.strip_prefix(".env.") {
        !suffix.is_empty()
            && suffix
                .chars()
                .any(|character| character.is_ascii_alphanumeric())
            && suffix.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
            })
    } else {
        false
    };
    if !valid {
        bail!("environment file must be `.env` or a `.env.<name>` basename: {name}");
    }

    let reserved = name
        .to_ascii_lowercase()
        .split(['.', '-', '_'])
        .any(|part| matches!(part, "prod" | "production" | "aws"));
    if reserved {
        bail!("production and AWS environment files cannot be selected: {name}");
    }
    Ok(())
}

fn regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_file())
}

fn create_private_dir_all(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("create directory {}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("secure directory {}", path.display()))
}

fn secure_write(path: &Path, contents: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .with_context(|| format!("secure {}", path.display()))?;
    file.write_all(contents)
        .with_context(|| format!("write {}", path.display()))
}

fn resolve_dev_command(
    repo: &TaskRepository,
    overrides: &RuntimeOverrides,
    provided: Vec<String>,
) -> Result<Vec<String>> {
    let command = if !provided.is_empty() {
        provided
    } else if let Some(RepoRuntime { dev_command }) = overrides.repos.get(&repo.name) {
        dev_command.clone()
    } else if has_package_dev_script(&repo.worktree_path)? {
        vec!["pnpm".to_owned(), "dev".to_owned()]
    } else {
        bail!(
            "no development command configured for {}; pass one after `--`",
            repo.name
        );
    };
    if command.is_empty() {
        bail!("runtime command for repository {} is empty", repo.name);
    }
    Ok(command
        .into_iter()
        .map(|part| part.replace("{port}", &repo.port.to_string()))
        .collect())
}

fn has_package_dev_script(repo: &Path) -> Result<bool> {
    let path = repo.join("package.json");
    if !path.is_file() {
        return Ok(false);
    }
    let contents = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let package: Value =
        serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
    Ok(package
        .get("scripts")
        .and_then(|scripts| scripts.get("dev"))
        .is_some_and(Value::is_string))
}

fn configured_command(command: &[String], repo: &TaskRepository, task_id: &str) -> Command {
    let mut process = Command::new(&command[0]);
    process
        .args(&command[1..])
        .current_dir(&repo.worktree_path)
        .env("PORT", repo.port.to_string())
        .env("KJ_TASK_ID", task_id)
        .env("KJ_REPO", &repo.name);
    process
}

fn ensure_process_success(status: ExitStatus, command: &[String]) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        bail!("command exited with {status}: {}", command.join(" "))
    }
}

pub fn process_alive(pid: u32) -> bool {
    let Ok(pid) = i32::try_from(pid) else {
        return false;
    };
    let result = unsafe { libc::kill(pid, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn managed_process_alive(pid: u32) -> bool {
    if !process_alive(pid) {
        return false;
    }
    i32::try_from(pid).is_ok_and(|pid| unsafe { libc::getpgid(pid) } == pid)
}

fn stop_process_group(pid: u32) -> Result<()> {
    if !process_alive(pid) {
        return Ok(());
    }
    let pid = i32::try_from(pid).context("PID exceeds platform range")?;
    let process_group = unsafe { libc::getpgid(pid) };
    if process_group != pid {
        bail!("refusing to stop PID {pid}: it is not a process-group leader started by kj");
    }
    let term_result = unsafe { libc::kill(-pid, libc::SIGTERM) };
    if term_result != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(error).context("send SIGTERM to process group");
        }
    }
    for _ in 0..50 {
        if !process_group_alive(pid) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    let kill_result = unsafe { libc::kill(-pid, libc::SIGKILL) };
    if kill_result != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(error).context("send SIGKILL to process group");
        }
    }
    Ok(())
}

fn process_group_alive(process_group: i32) -> bool {
    let result = unsafe { libc::kill(-process_group, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn tail_file(path: &Path, max_lines: usize) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let length = file.metadata()?.len();
    let read_length = length.min(128 * 1024);
    file.seek(SeekFrom::End(-(read_length as i64)))?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .with_context(|| format!("read {}", path.display()))?;
    let lines = buffer.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(max_lines);
    Ok(lines[start..].join("\n"))
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn validate_task_id(value: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= 80
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && !value.ends_with(".lock")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        });
    if !valid {
        bail!("invalid task ID; use 1-80 ASCII letters, numbers, dots, dashes, or underscores");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TaskRepository;
    use std::net::TcpListener;

    #[test]
    fn task_ids_reject_paths_and_git_lock_names() {
        for value in ["", "../escape", ".hidden", "task.lock", "a/b", "x..y"] {
            assert!(validate_task_id(value).is_err(), "{value} should fail");
        }
        for value in ["feature-auth", "task_name", "release.1"] {
            assert!(validate_task_id(value).is_ok(), "{value} should pass");
        }
    }

    #[test]
    fn codex_arguments_are_stable_and_sorted() {
        let manifest = TaskManifest {
            version: TASK_SCHEMA_VERSION,
            id: "feature-auth".to_owned(),
            branch: "codex/feature-auth".to_owned(),
            created_at: 0,
            environment_files: default_environment_files(),
            repositories: vec![
                task_repo("zeta", "/tmp/zeta"),
                task_repo("frontend", "/tmp/frontend"),
                task_repo("beta", "/tmp/beta"),
            ],
            processes: BTreeMap::new(),
        };
        let args = codex_arguments(&manifest, Some("frontend"), &["fix".to_owned()]).unwrap();
        assert_eq!(
            args,
            vec![
                "-C",
                "/tmp/frontend",
                "--sandbox",
                "workspace-write",
                "--add-dir",
                "/tmp/beta",
                "--add-dir",
                "/tmp/zeta",
                "fix"
            ]
        );
    }

    #[test]
    fn occupied_port_is_not_available() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(ensure_port_available(port).is_err());
    }

    #[test]
    fn runtime_override_replaces_the_reserved_port() {
        let repo = task_repo("frontend", "/tmp/frontend");
        let overrides = RuntimeOverrides {
            repos: BTreeMap::from([(
                "frontend".to_owned(),
                RepoRuntime {
                    dev_command: vec![
                        "pnpm".to_owned(),
                        "dev".to_owned(),
                        "--port".to_owned(),
                        "{port}".to_owned(),
                    ],
                },
            )]),
        };
        assert_eq!(
            resolve_dev_command(&repo, &overrides, Vec::new()).unwrap(),
            ["pnpm", "dev", "--port", "41000"]
        );
    }

    #[test]
    fn pid_files_require_a_plain_unsigned_integer() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("service.pid");
        fs::write(&path, "12345\n").unwrap();
        assert_eq!(read_pid_file(&path).unwrap(), 12_345);
        fs::write(&path, "123 command").unwrap();
        assert!(read_pid_file(&path).is_err());
    }

    fn task_repo(name: &str, path: &str) -> TaskRepository {
        TaskRepository {
            name: name.to_owned(),
            canonical_path: PathBuf::from(path),
            worktree_path: PathBuf::from(path),
            base_ref: "origin/main".to_owned(),
            base_sha: "abc".to_owned(),
            port: 41_000,
        }
    }
}
