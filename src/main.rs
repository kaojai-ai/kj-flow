use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand, error::ErrorKind};
use kj_flow::app;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(
    name = "kj",
    version,
    about = "Human-agent development workflows by KaoJai"
)]
struct Cli {
    #[arg(long, global = true, help = "Emit a stable JSON envelope")]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Configure a multi-repository workspace")]
    Init {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(
            long,
            help = "Optional absolute path or workspace-relative worktree root"
        )]
        worktree_root: Option<PathBuf>,
    },
    #[command(about = "Check local configuration and dependencies")]
    Doctor,
    #[command(subcommand, about = "Discover workspace repositories")]
    Repo(RepoCommands),
    #[command(subcommand, about = "Create and operate isolated tasks")]
    Task(TaskCommands),
}

#[derive(Debug, Subcommand)]
enum RepoCommands {
    #[command(about = "List immediate child Git repositories")]
    List,
}

#[derive(Debug, Subcommand)]
enum TaskCommands {
    #[command(about = "Create isolated worktrees for a task")]
    Create {
        task_id: String,
        #[arg(long = "repo", required = true)]
        repositories: Vec<String>,
    },
    #[command(about = "List managed tasks")]
    List,
    #[command(about = "Show one task")]
    Show { task_id: String },
    #[command(about = "Launch Codex in a task")]
    Codex {
        task_id: String,
        #[arg(long)]
        primary: Option<String>,
        #[arg(last = true, allow_hyphen_values = true)]
        extra: Vec<String>,
    },
    #[command(about = "Start a repository's development process")]
    Start {
        task_id: String,
        repository: String,
        #[arg(long)]
        foreground: bool,
        #[arg(last = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    #[command(about = "Read or follow a repository's task log")]
    Logs {
        task_id: String,
        repository: String,
        #[arg(long)]
        follow: bool,
    },
    #[command(about = "Stop task development processes")]
    Stop {
        task_id: String,
        repository: Option<String>,
    },
    #[command(about = "Run configured task resource cleanup without removing worktrees")]
    Cleanup {
        task_id: String,
        repository: Option<String>,
    },
    #[command(subcommand, about = "Manage task-local environment files")]
    Env(TaskEnvCommands),
    #[command(about = "Preview or remove completed task worktrees")]
    Finish {
        task_id: String,
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Debug, Subcommand)]
enum TaskEnvCommands {
    #[command(about = "Preview or refresh safe local environment files")]
    Sync {
        task_id: String,
        repository: String,
        #[arg(long)]
        apply: bool,
    },
}

fn main() -> ExitCode {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    let requested_json = arguments.iter().any(|argument| argument == "--json");
    let cli = match Cli::try_parse_from(arguments) {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            if requested_json {
                emit_success(json!({ "text": error.to_string() }), true);
            } else {
                let _ = error.print();
            }
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            if requested_json {
                emit_error(&error.to_string(), true);
            } else {
                let _ = error.print();
            }
            return ExitCode::from(2);
        }
    };
    let json_mode = cli.json;
    match execute(cli) {
        Ok(value) => {
            emit_success(value, json_mode);
            ExitCode::SUCCESS
        }
        Err(error) => {
            emit_error(&format!("{error:#}"), json_mode);
            ExitCode::FAILURE
        }
    }
}

fn execute(cli: Cli) -> Result<Value> {
    let json_mode = cli.json;
    match cli.command {
        Commands::Init {
            workspace,
            worktree_root,
        } => app::init(workspace, worktree_root),
        Commands::Doctor => Ok(app::doctor()),
        Commands::Repo(RepoCommands::List) => app::repo_list(),
        Commands::Task(task) => match task {
            TaskCommands::Create {
                task_id,
                repositories,
            } => app::task_create(&task_id, &repositories),
            TaskCommands::List => app::task_list(),
            TaskCommands::Show { task_id } => app::task_show(&task_id),
            TaskCommands::Codex {
                task_id,
                primary,
                extra,
            } => {
                if json_mode {
                    anyhow::bail!("--json cannot be combined with `task codex`");
                }
                app::task_codex(&task_id, primary.as_deref(), extra)
            }
            TaskCommands::Start {
                task_id,
                repository,
                foreground,
                command,
            } => {
                if json_mode && foreground {
                    anyhow::bail!("--json cannot be combined with --foreground");
                }
                app::task_start(&task_id, &repository, foreground, command)
            }
            TaskCommands::Logs {
                task_id,
                repository,
                follow,
            } => app::task_logs(&task_id, &repository, follow, json_mode),
            TaskCommands::Stop {
                task_id,
                repository,
            } => app::task_stop(&task_id, repository.as_deref()),
            TaskCommands::Cleanup {
                task_id,
                repository,
            } => app::task_cleanup(&task_id, repository.as_deref()),
            TaskCommands::Env(TaskEnvCommands::Sync {
                task_id,
                repository,
                apply,
            }) => app::task_env_sync(&task_id, &repository, apply),
            TaskCommands::Finish { task_id, apply } => app::task_finish(&task_id, apply),
        },
    }
}

fn emit_success(data: Value, json_mode: bool) {
    if json_mode {
        println!(
            "{}",
            serde_json::to_string(&json!({ "ok": true, "data": data }))
                .expect("serialize success envelope")
        );
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&data).expect("serialize human output")
        );
    }
}

fn emit_error(message: &str, json_mode: bool) {
    if json_mode {
        println!(
            "{}",
            serde_json::to_string(&json!({
                "ok": false,
                "error": {
                    "code": "command_failed",
                    "message": message
                }
            }))
            .expect("serialize error envelope")
        );
    } else {
        eprintln!("error: {message}");
    }
}

#[allow(dead_code)]
fn command_for_docs() -> clap::Command {
    Cli::command()
}
