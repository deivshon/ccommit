//! Spawns `git`, so that hooks and `commit.gpgsign` keep working.

use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use crate::message::ConventionalCommit;

pub fn is_repository(path: &Path) -> bool {
    run_quiet(path, &["rev-parse", "--is-inside-work-tree"]).is_some_and(|status| status.success())
}

pub fn has_staged_changes(path: &Path) -> bool {
    if head_exists(path) {
        // git exits 1 when the index differs from HEAD.
        run_quiet(path, &["diff-index", "--cached", "--quiet", "HEAD"])
            .is_some_and(|status| status.code() == Some(1))
    } else {
        // HEAD is unborn, so every index entry is a staged change.
        index_is_not_empty(path)
    }
}

/// The part of the git identity that `git commit` cannot find.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingIdentity {
    Name,
    Email,
    Both,
}

/// `git var` builds the identity that `git commit` needs, so it fails for the same reasons.
pub fn missing_identity(path: &Path) -> Option<MissingIdentity> {
    if identity_is_known(path, "GIT_AUTHOR_IDENT") && identity_is_known(path, "GIT_COMMITTER_IDENT")
    {
        return None;
    }

    match (
        has_setting(path, "user.name"),
        has_setting(path, "user.email"),
    ) {
        (false, true) => Some(MissingIdentity::Name),
        (true, false) => Some(MissingIdentity::Email),
        _ => Some(MissingIdentity::Both),
    }
}

pub fn commit(path: &Path, message: &ConventionalCommit) -> Result<(), String> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(path)
        .arg("commit")
        .arg("-m")
        .arg(&message.short_message);

    if !message.long_message.is_empty() {
        command.arg("-m").arg(&message.long_message);
    }

    let status = command
        .status()
        .map_err(|error| format!("Could not run git: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("git commit failed with {}", describe(status)))
    }
}

fn identity_is_known(path: &Path, variable: &str) -> bool {
    run_quiet(path, &["var", variable]).is_some_and(|status| status.success())
}

fn has_setting(path: &Path, key: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["config", "--get", key])
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|output| output.status.success() && !output.stdout.trim_ascii().is_empty())
}

fn head_exists(path: &Path) -> bool {
    run_quiet(path, &["rev-parse", "--verify", "HEAD"]).is_some_and(|status| status.success())
}

fn index_is_not_empty(path: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["ls-files", "--cached"])
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|output| output.status.success() && !output.stdout.is_empty())
}

fn run_quiet(path: &Path, arguments: &[&str]) -> Option<ExitStatus> {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(arguments)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
}

fn describe(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("exit code {code}"),
        None => "a signal".to_string(),
    }
}
