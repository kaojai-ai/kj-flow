use crate::model::{RuntimeOverrides, UserConfig};
use anyhow::{Context, Result, bail};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

pub fn config_path() -> Result<PathBuf> {
    if let Some(path) = env::var_os("KJ_CONFIG_PATH") {
        return Ok(PathBuf::from(path));
    }
    let home = dirs::home_dir().context("cannot determine the user home directory")?;
    Ok(home.join(".config").join("kj-flow").join("config.toml"))
}

pub fn save_user_config(config: &UserConfig) -> Result<PathBuf> {
    let path = config_path()?;
    let parent = path
        .parent()
        .context("configuration path has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("create config directory {}", parent.display()))?;
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("secure config directory {}", parent.display()))?;
    let contents = toml::to_string_pretty(config).context("serialize user configuration")?;
    let temp_path = parent.join("config.toml.tmp");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(&temp_path)
        .with_context(|| format!("open {}", temp_path.display()))?;
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .with_context(|| format!("secure {}", temp_path.display()))?;
    file.write_all(contents.as_bytes())
        .with_context(|| format!("write {}", temp_path.display()))?;
    fs::rename(&temp_path, &path).with_context(|| format!("replace {}", path.display()))?;
    Ok(path)
}

pub fn load_user_config() -> Result<UserConfig> {
    if let Some(root) = env::var_os("KJ_WORKSPACE_ROOT") {
        return validate_config(UserConfig::new(PathBuf::from(root)));
    }
    let path = config_path()?;
    let contents = fs::read_to_string(&path).with_context(|| {
        format!(
            "missing configuration {}; run `kj init --workspace <path>`",
            path.display()
        )
    })?;
    let config: UserConfig =
        toml::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
    validate_config(config)
}

pub fn validate_config(mut config: UserConfig) -> Result<UserConfig> {
    if !config.workspace_root.is_dir() {
        bail!(
            "workspace does not exist: {}",
            config.workspace_root.display()
        );
    }
    config.workspace_root = config
        .workspace_root
        .canonicalize()
        .context("canonicalize workspace path")?;
    if config.workspace_root.join(".git").exists() {
        bail!(
            "workspace root must be a non-Git container: {}",
            config.workspace_root.display()
        );
    }
    if config.worktree_root.as_ref().is_some_and(|path| {
        path.is_relative()
            && path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
    }) {
        bail!("relative worktree_root must stay inside workspace_root");
    }
    if config.worktree_root() == config.workspace_root {
        bail!("worktree_root must not be the workspace root");
    }
    if config.port_start < 41_000 || config.port_end > 49_999 || config.port_start > config.port_end
    {
        bail!(
            "invalid port range {}-{}",
            config.port_start,
            config.port_end
        );
    }
    let branch_prefix_valid = !config.branch_prefix.is_empty()
        && config.branch_prefix.len() <= 40
        && config
            .branch_prefix
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'));
    if !branch_prefix_valid {
        bail!("invalid branch_prefix; use 1-40 ASCII letters, numbers, dashes, or underscores");
    }
    Ok(config)
}

pub fn load_runtime_overrides(workspace: &Path) -> Result<RuntimeOverrides> {
    let user_path = config_path()?
        .parent()
        .context("configuration path has no parent directory")?
        .join("repos.toml");
    let paths = [workspace.join(".kj").join("repos.toml"), user_path];
    let mut merged = RuntimeOverrides::default();
    for path in paths {
        if !path.exists() {
            continue;
        }
        let contents =
            fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let overrides: RuntimeOverrides =
            toml::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
        merged.repos.extend(overrides.repos);
    }
    for (name, runtime) in &merged.repos {
        if runtime.dev_command.is_empty() || runtime.dev_command[0].is_empty() {
            bail!("runtime override for {name} has an empty dev_command");
        }
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn user_runtime_commands_override_workspace_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let workspace = directory.path().join("workspace");
        fs::create_dir_all(workspace.join(".kj")).unwrap();
        fs::write(
            workspace.join(".kj/repos.toml"),
            "[repos.frontend]\ndev_command = [\"pnpm\", \"dev\"]\n",
        )
        .unwrap();
        fs::write(
            directory.path().join("repos.toml"),
            "[repos.frontend]\ndev_command = [\"npm\", \"run\", \"dev\"]\n\
             [repos.api]\ndev_command = [\"cargo\", \"run\"]\n",
        )
        .unwrap();
        let config = directory.path().join("config.toml");
        // SAFETY: this test is serialized and restores the process environment.
        unsafe { env::set_var("KJ_CONFIG_PATH", &config) };

        let overrides = load_runtime_overrides(&workspace).unwrap();

        unsafe { env::remove_var("KJ_CONFIG_PATH") };
        assert_eq!(
            overrides.repos["frontend"].dev_command,
            ["npm", "run", "dev"]
        );
        assert_eq!(overrides.repos["api"].dev_command, ["cargo", "run"]);
    }
}
