use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const TASK_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub workspace_root: PathBuf,
    #[serde(default)]
    pub worktree_root: Option<PathBuf>,
    #[serde(default = "default_port_start")]
    pub port_start: u16,
    #[serde(default = "default_port_end")]
    pub port_end: u16,
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
}

impl UserConfig {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            worktree_root: None,
            port_start: default_port_start(),
            port_end: default_port_end(),
            branch_prefix: default_branch_prefix(),
        }
    }

    pub fn worktree_root(&self) -> PathBuf {
        match &self.worktree_root {
            Some(path) if path.is_absolute() => path.clone(),
            Some(path) => self.workspace_root.join(path),
            None => self.workspace_root.join("worktrees"),
        }
    }
}

fn default_port_start() -> u16 {
    41_000
}

fn default_port_end() -> u16 {
    49_999
}

fn default_branch_prefix() -> String {
    "codex".to_owned()
}

#[derive(Debug, Clone, Serialize)]
pub struct RepositoryInfo {
    pub name: String,
    pub path: PathBuf,
    pub origin: Option<String>,
    pub default_branch: String,
    pub runtime_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManifest {
    pub version: u32,
    pub id: String,
    pub branch: String,
    pub created_at: u64,
    pub repositories: Vec<TaskRepository>,
    #[serde(default)]
    pub processes: BTreeMap<String, ProcessRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRepository {
    pub name: String,
    pub canonical_path: PathBuf,
    pub worktree_path: PathBuf,
    pub base_ref: String,
    pub base_sha: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRecord {
    pub pid: u32,
    pub started_at: u64,
    pub log_path: PathBuf,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RuntimeOverrides {
    #[serde(default)]
    pub repos: BTreeMap<String, RepoRuntime>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RepoRuntime {
    #[serde(default)]
    pub dev_command: Vec<String>,
    #[serde(default)]
    pub cleanup_command: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}
