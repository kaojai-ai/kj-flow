use crate::model::{RepositoryInfo, RuntimeOverrides};
use anyhow::{Context, Result, bail};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn discover_repositories(
    workspace: &Path,
    overrides: &RuntimeOverrides,
) -> Result<Vec<RepositoryInfo>> {
    let mut repositories = Vec::new();
    for entry in std::fs::read_dir(workspace)
        .with_context(|| format!("read workspace {}", workspace.display()))?
    {
        let entry = entry.context("read workspace entry")?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if !path.is_dir() || name.starts_with('.') || name == "worktrees" {
            continue;
        }
        let output = run_output(&path, ["rev-parse", "--show-toplevel"]);
        let Ok(output) = output else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let reported = PathBuf::from(stdout_trimmed(&output)?);
        let Ok(reported) = reported.canonicalize() else {
            continue;
        };
        let Ok(canonical) = path.canonicalize() else {
            continue;
        };
        if reported != canonical {
            continue;
        }
        let origin = optional_stdout(&path, ["remote", "get-url", "origin"])
            .map(|url| sanitize_remote_url(&url));
        let default_branch = remote_default_branch(&path)
            .or_else(|| current_branch(&path))
            .unwrap_or_else(|| "main".to_owned());
        repositories.push(RepositoryInfo {
            runtime_configured: overrides
                .repos
                .get(&name)
                .is_some_and(|runtime| !runtime.dev_command.is_empty()),
            name,
            path: canonical,
            origin,
            default_branch,
        });
    }
    repositories.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(repositories)
}

pub fn prepare_base(repo: &RepositoryInfo) -> Result<(String, String)> {
    let base_ref = if repo.origin.is_some() {
        let default_branch = remote_head_branch(&repo.path)
            .with_context(|| format!("resolve origin/HEAD for {}", repo.name))?;
        run_checked(
            &repo.path,
            ["fetch", "--prune", "origin", default_branch.as_str()],
        )
        .with_context(|| format!("fetch {} {default_branch}", repo.name))?;
        format!("origin/{default_branch}")
    } else {
        repo.default_branch.clone()
    };
    let sha = checked_stdout(&repo.path, ["rev-parse", base_ref.as_str()])
        .with_context(|| format!("resolve base for {}", repo.name))?;
    Ok((base_ref, sha))
}

pub fn ensure_branch_available(repo: &RepositoryInfo, branch: &str) -> Result<()> {
    let local = format!("refs/heads/{branch}");
    if run_output(
        &repo.path,
        ["show-ref", "--verify", "--quiet", local.as_str()],
    )?
    .status
    .success()
    {
        bail!("branch already exists in {}: {}", repo.name, branch);
    }
    let remote = format!("refs/remotes/origin/{branch}");
    if run_output(
        &repo.path,
        ["show-ref", "--verify", "--quiet", remote.as_str()],
    )?
    .status
    .success()
    {
        bail!("remote branch already exists in {}: {}", repo.name, branch);
    }
    if repo.origin.is_some() {
        let output = run_output(
            &repo.path,
            ["ls-remote", "--exit-code", "--heads", "origin", branch],
        )?;
        match output.status.code() {
            Some(0) => bail!("remote branch already exists in {}: {}", repo.name, branch),
            Some(2) => {}
            _ => {
                ensure_success(output)
                    .with_context(|| format!("check remote branch in {}", repo.name))?;
            }
        }
    }
    Ok(())
}

pub fn add_worktree(
    repo: &RepositoryInfo,
    path: &Path,
    branch: &str,
    base_ref: &str,
) -> Result<()> {
    run_checked_os(
        &repo.path,
        [
            OsStr::new("worktree"),
            OsStr::new("add"),
            OsStr::new("-b"),
            OsStr::new(branch),
            path.as_os_str(),
            OsStr::new(base_ref),
        ],
    )
    .with_context(|| format!("create worktree {}", path.display()))
}

pub fn remove_worktree(canonical_repo: &Path, worktree: &Path, force: bool) -> Result<()> {
    let mut args = vec![OsStr::new("worktree"), OsStr::new("remove")];
    if force {
        args.push(OsStr::new("--force"));
    }
    args.push(worktree.as_os_str());
    run_checked_os(canonical_repo, args)
        .with_context(|| format!("remove worktree {}", worktree.display()))
}

pub fn delete_local_branch(canonical_repo: &Path, branch: &str) -> Result<()> {
    run_checked(canonical_repo, ["branch", "-D", branch])
}

pub fn worktree_clean(worktree: &Path) -> Result<bool> {
    Ok(checked_stdout(worktree, ["status", "--porcelain"])?.is_empty())
}

pub fn path_is_tracked(worktree: &Path, relative_path: &str) -> Result<bool> {
    Ok(run_output(
        worktree,
        ["ls-files", "--error-unmatch", "--", relative_path],
    )?
    .status
    .success())
}

pub fn head_sha(worktree: &Path) -> Result<String> {
    checked_stdout(worktree, ["rev-parse", "HEAD"])
}

pub fn commits_are_pushed(worktree: &Path, branch: &str, base_sha: &str) -> Result<bool> {
    let head = head_sha(worktree)?;
    if head == base_sha {
        return Ok(true);
    }
    if optional_stdout(worktree, ["remote", "get-url", "origin"]).is_none() {
        return Ok(true);
    }
    if !run_output(worktree, ["fetch", "--prune", "origin"])?
        .status
        .success()
    {
        return Ok(false);
    }
    let remote_ref = format!("origin/{branch}");
    Ok(
        optional_stdout(worktree, ["rev-parse", remote_ref.as_str()])
            .is_some_and(|remote_sha| remote_sha == head),
    )
}

pub fn current_branch(repo: &Path) -> Option<String> {
    optional_stdout(repo, ["branch", "--show-current"]).filter(|value| !value.is_empty())
}

fn remote_default_branch(repo: &Path) -> Option<String> {
    optional_stdout(
        repo,
        ["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    )
    .and_then(|value| value.strip_prefix("origin/").map(str::to_owned))
}

fn remote_head_branch(repo: &Path) -> Result<String> {
    let output = ensure_success(run_output(
        repo,
        ["ls-remote", "--symref", "origin", "HEAD"],
    )?)?;
    let stdout = stdout_trimmed(&output)?;
    stdout
        .lines()
        .find_map(|line| {
            line.strip_prefix("ref: refs/heads/")
                .and_then(|line| line.strip_suffix("\tHEAD"))
                .map(str::to_owned)
        })
        .filter(|branch| !branch.is_empty())
        .context("origin/HEAD is not a symbolic branch")
}

pub fn checked_stdout<I, S>(repo: &Path, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run_output(repo, args)?;
    ensure_success(output).and_then(|value| stdout_trimmed(&value))
}

pub fn run_checked<I, S>(repo: &Path, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_checked_os(repo, args)
}

fn run_checked_os<I, S>(repo: &Path, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run_output(repo, args)?;
    ensure_success(output).map(|_| ())
}

fn run_output<I, S>(repo: &Path, args: I) -> Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .with_context(|| format!("run git in {}", repo.display()))
}

fn ensure_success(output: Output) -> Result<Output> {
    if output.status.success() {
        return Ok(output);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    bail!(
        "git exited with {}{}",
        output.status,
        if stderr.is_empty() {
            String::new()
        } else {
            format!(": {stderr}")
        }
    )
}

fn stdout_trimmed(output: &Output) -> Result<String> {
    String::from_utf8(output.stdout.clone())
        .context("git output was not UTF-8")
        .map(|value| value.trim().to_owned())
}

fn optional_stdout<I, S>(repo: &Path, args: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run_output(repo, args).ok()?;
    if !output.status.success() {
        return None;
    }
    stdout_trimmed(&output).ok()
}

fn sanitize_remote_url(url: &str) -> String {
    let Some((scheme, remainder)) = url.split_once("://") else {
        return url.to_owned();
    };
    let (authority, path) = remainder
        .split_once('/')
        .map_or((remainder, None), |(authority, path)| {
            (authority, Some(path))
        });
    let sanitized_authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    match path {
        Some(path) => format!("{scheme}://{sanitized_authority}/{path}"),
        None => format!("{scheme}://{sanitized_authority}"),
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_remote_url;

    #[test]
    fn remote_urls_never_expose_http_credentials() {
        assert_eq!(
            sanitize_remote_url("https://user:secret@example.com/team/repo.git"),
            "https://example.com/team/repo.git"
        );
        assert_eq!(
            sanitize_remote_url("git@example.com:team/repo.git"),
            "git@example.com:team/repo.git"
        );
        assert_eq!(
            sanitize_remote_url("https://example.com/team/repo@release.git"),
            "https://example.com/team/repo@release.git"
        );
    }
}
