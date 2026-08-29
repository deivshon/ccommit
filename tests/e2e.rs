//! Runs the real binary in a temporary repository under a pseudo-terminal.
//!
//! This tier exists for the chain the wizard tests mock away: a real hook fires, a real
//! `git commit` runs, and the real message lands.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

const BINARY: &str = env!("CARGO_BIN_EXE_ccommit");
const KEY_DELAY: Duration = Duration::from_millis(150);
const EXIT_TIMEOUT: Duration = Duration::from_secs(10);

struct Repository {
    path: PathBuf,
    config_home: PathBuf,
}

impl Repository {
    /// A directory with no repository inside, isolated from the user's git and ccommit config.
    fn bare(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("ccommit-e2e-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();

        let path = root.join("repo");
        let config_home = root.join("config");
        std::fs::create_dir_all(&path).expect("the repository directory is created");
        std::fs::create_dir_all(&config_home).expect("the config directory is created");

        Self { path, config_home }
    }

    /// An empty repository that does not read the user's git config.
    fn empty(name: &str) -> Self {
        let repository = Self::bare(name);
        repository.git(&["init", "--quiet"]);
        repository.git(&["config", "user.name", "ccommit test"]);
        repository.git(&["config", "user.email", "test@example.com"]);
        repository.git(&["config", "commit.gpgsign", "false"]);
        repository
    }

    /// One staged file and an unborn HEAD.
    fn unborn(name: &str) -> Self {
        let repository = Self::empty(name);
        repository.stage("one.txt", "one\n");
        repository
    }

    /// One commit, plus one staged file.
    fn with_history(name: &str) -> Self {
        let repository = Self::empty(name);
        repository.stage("readme.txt", "readme\n");
        repository.git(&["commit", "--quiet", "-m", "chore: first"]);
        repository.stage("one.txt", "one\n");
        repository
    }

    /// One staged file, and no identity for `git commit`.
    fn without_identity(name: &str) -> Self {
        let repository = Self::bare(name);
        repository.git(&["init", "--quiet"]);
        repository.git(&["config", "commit.gpgsign", "false"]);
        // Without this key, git guesses a name and an email from the system.
        repository.git(&["config", "user.useConfigOnly", "true"]);
        repository.stage("one.txt", "one\n");
        repository
    }

    /// One commit and nothing staged.
    fn clean(name: &str) -> Self {
        let repository = Self::empty(name);
        repository.stage("readme.txt", "readme\n");
        repository.git(&["commit", "--quiet", "-m", "chore: first"]);
        repository
    }

    fn stage(&self, name: &str, contents: &str) {
        std::fs::write(self.path.join(name), contents).expect("the file is written");
        self.git(&["add", name]);
    }

    fn git(&self, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .args(arguments)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("git runs");

        assert!(
            output.status.success(),
            "git {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    fn last_message(&self) -> String {
        self.git(&["log", "-1", "--pretty=%B"])
            .trim_end()
            .to_string()
    }

    fn commit_count(&self) -> usize {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .args(["rev-list", "--count", "HEAD"])
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("git runs");

        if output.status.success() {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse()
                .unwrap_or(0)
        } else {
            0
        }
    }

    /// The files the commit at HEAD holds.
    fn files_in_head(&self) -> Vec<String> {
        self.git(&["ls-tree", "-r", "--name-only", "HEAD"])
            .lines()
            .map(str::to_string)
            .collect()
    }

    fn add_failing_pre_commit_hook(&self) {
        self.add_hook("pre-commit", "#!/bin/sh\necho hook says no >&2\nexit 1\n");
    }

    fn add_hook(&self, name: &str, body: &str) {
        let hook = self.path.join(".git/hooks").join(name);
        std::fs::write(&hook, body).expect("the hook is written");
        set_executable(&hook);
    }

    /// Writes a file inside the repository, creating the parent directories.
    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.path.join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent"))
            .expect("the directory is created");
        std::fs::write(&path, contents).expect("the file is written");
        path
    }

    fn write_config(&self, contents: &str) -> PathBuf {
        let path = self.config_home.join("ccommit/config.json");
        std::fs::create_dir_all(path.parent().expect("a parent"))
            .expect("the directory is created");
        std::fs::write(&path, contents).expect("the config file is written");
        path
    }

    /// Runs the binary with no pseudo-terminal, for the paths that fail before the wizard.
    fn run_bare(&self, arguments: &[&str]) -> std::process::Output {
        Command::new(BINARY)
            .arg("-C")
            .arg(&self.path)
            .args(arguments)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("the binary runs")
    }

    /// Spawns the binary under a pseudo-terminal, types the keys, and waits for it to exit.
    fn run(&self, arguments: &[&str], keys: &[&[u8]]) -> Outcome {
        let directory = self.path.clone();
        self.run_from(&directory, arguments, keys)
    }

    /// The same, with the process directory somewhere else than the target repository.
    fn run_from(&self, current: &Path, arguments: &[&str], keys: &[&[u8]]) -> Outcome {
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows: 30,
                cols: 100,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("the pseudo-terminal opens");

        let mut command = CommandBuilder::new(BINARY);
        command.arg("-C");
        command.arg(&self.path);
        for argument in arguments {
            command.arg(argument);
        }
        command.cwd(current);
        command.env("XDG_CONFIG_HOME", &self.config_home);
        command.env("GIT_CONFIG_GLOBAL", "/dev/null");
        command.env("GIT_CONFIG_SYSTEM", "/dev/null");
        command.env("TERM", "xterm-256color");

        let mut child = pair
            .slave
            .spawn_command(command)
            .expect("the binary starts");
        drop(pair.slave);

        let output = Arc::new(Mutex::new(Vec::new()));
        let mut reader = pair
            .master
            .try_clone_reader()
            .expect("the reader is cloned");
        let collected = Arc::clone(&output);
        let pump = std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            while let Ok(read) = reader.read(&mut chunk) {
                if read == 0 {
                    break;
                }
                collected
                    .lock()
                    .expect("the buffer is free")
                    .extend_from_slice(&chunk[..read]);
            }
        });

        let mut writer = pair.master.take_writer().expect("the writer opens");

        // Until the app enables raw mode, the pty is still in canonical mode and would mangle the
        // keys. The alternate screen sequence proves that raw mode is on.
        wait_for(&output, ENTER_ALTERNATE_SCREEN);

        for key in keys {
            writer.write_all(key).expect("the key is written");
            writer.flush().expect("the key reaches the terminal");
            std::thread::sleep(KEY_DELAY);
        }

        // A wizard that never finishes would block forever, so the wait carries a deadline.
        let deadline = std::time::Instant::now() + EXIT_TIMEOUT;
        let status = loop {
            match child.try_wait().expect("the child is readable") {
                Some(status) => break Some(status),
                None if std::time::Instant::now() >= deadline => {
                    child.kill().ok();
                    child.wait().ok();
                    break None;
                }
                None => std::thread::sleep(Duration::from_millis(50)),
            }
        };

        drop(writer);
        drop(pair.master);
        pump.join().ok();

        let text = String::from_utf8_lossy(&output.lock().expect("the buffer is free")).to_string();
        let status = status.unwrap_or_else(|| {
            panic!("the binary did not exit within {EXIT_TIMEOUT:?}. Screen:\n{text}")
        });

        Outcome {
            code: status.exit_code(),
            text,
        }
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        if let Some(root) = self.path.parent() {
            std::fs::remove_dir_all(root).ok();
        }
    }
}

struct Outcome {
    code: u32,
    text: String,
}

const ENTER_ALTERNATE_SCREEN: &[u8] = b"\x1b[?1049h";

/// Blocks until the collected output holds the needle.
fn wait_for(output: &Arc<Mutex<Vec<u8>>>, needle: &[u8]) {
    let deadline = std::time::Instant::now() + EXIT_TIMEOUT;

    loop {
        let found = output
            .lock()
            .expect("the buffer is free")
            .windows(needle.len())
            .any(|window| window == needle);

        if found {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the alternate screen never opened"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
fn set_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)
        .expect("the hook exists")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("the hook becomes executable");
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) {}

const ENTER: &[u8] = b"\r";
const SHIFT_ENTER: &[u8] = b"\x1b[13;2u";
const OPTION_ENTER: &[u8] = b"\x1b\r";
const ESC: &[u8] = b"\x1b";
const CTRL_C: &[u8] = b"\x03";
const DOWN: &[u8] = b"\x1b[B";
const CTRL_U: &[u8] = b"\x15";

/// `feat`, no scope, `:sparkles:`, then the description.
fn wizard_keys() -> Vec<&'static [u8]> {
    vec![
        b"feat",
        ENTER,
        ENTER,
        b":sparkles:",
        ENTER,
        b"add thing",
        ENTER,
    ]
}

#[test]
fn the_wizard_writes_a_real_commit() {
    let repository = Repository::with_history("commit");

    let mut keys = wizard_keys();
    keys.extend([b"the body".as_slice(), ENTER, ENTER, ENTER]);
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(
        repository.last_message(),
        "feat: ✨ add thing\n\nthe body",
        "{}",
        outcome.text
    );
}

#[test]
fn the_wizard_creates_the_first_commit_in_a_new_repository() {
    let repository = Repository::unborn("unborn");

    let mut keys = wizard_keys();
    keys.extend([ENTER, ENTER, ENTER]);
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.commit_count(), 1, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: ✨ add thing");
}

#[test]
fn the_message_reaches_stdout_and_no_commit_happens() {
    let repository = Repository::with_history("print");

    let mut keys = wizard_keys();
    keys.extend([ENTER, ENTER, ENTER]);
    let outcome = repository.run(&["--commit-on-finish=off"], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert!(
        outcome.text.contains("feat: ✨ add thing"),
        "{}",
        outcome.text
    );
    assert_eq!(repository.commit_count(), 1, "no commit was added");
}

#[test]
fn a_failing_pre_commit_hook_stops_the_commit() {
    let repository = Repository::with_history("hook");
    repository.add_failing_pre_commit_hook();

    let mut keys = wizard_keys();
    keys.extend([ENTER, ENTER, ENTER]);
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 1, "{}", outcome.text);
    assert!(outcome.text.contains("hook says no"), "{}", outcome.text);
    assert_eq!(repository.commit_count(), 1, "the hook stopped the commit");
}

#[test]
fn the_gitmoji_code_lands_in_the_message() {
    let repository = Repository::with_history("code");

    let mut keys = wizard_keys();
    keys.extend([ENTER, ENTER]);
    let outcome = repository.run(&["--real-emoji=off", "--preview-on-finish=off"], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: :sparkles: add thing");
}

#[test]
fn esc_on_the_first_menu_leaves_the_repository_alone() {
    let repository = Repository::with_history("quit");

    let outcome = repository.run(&[], &[ESC]);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.commit_count(), 1);
}

#[test]
fn a_directory_outside_a_repository_fails() {
    let repository = Repository::bare("outside");

    let output = repository.run_bare(&[]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "Failure: Can't commit outside of a git repository"
    );
}

#[test]
fn a_repository_with_no_staged_changes_fails() {
    let repository = Repository::clean("empty");

    let output = repository.run_bare(&[]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "Failure: Can't commit without changes added"
    );
}

#[test]
fn a_repository_without_an_identity_fails_before_the_wizard() {
    let repository = Repository::without_identity("identity");

    let output = repository.run_bare(&[]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "Failure: Can't commit without a git identity. Set it with:\n  \
         git config user.name \"Your Name\"\n  \
         git config user.email \"you@example.com\""
    );
    assert_eq!(repository.commit_count(), 0);
}

#[test]
fn a_direct_commit_without_an_identity_fails() {
    let repository = Repository::without_identity("identity-direct");

    let output = repository.run_bare(&["-s", "feat: add thing"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("without a git identity"), "{stderr}");
    assert_eq!(repository.commit_count(), 0);
}

/// The name is there, so only the email reaches the message.
#[test]
fn a_missing_email_names_the_email_alone() {
    let repository = Repository::without_identity("identity-email");
    repository.git(&["config", "user.name", "ccommit test"]);

    let output = repository.run_bare(&["-s", "feat: add thing"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "Failure: Can't commit without a git identity. Set it with:\n  \
         git config user.email \"you@example.com\""
    );
}

#[test]
fn a_print_without_a_commit_needs_no_identity() {
    let repository = Repository::without_identity("identity-print");

    let output = repository.run_bare(&["-s", "feat: add thing", "--commit-on-finish=off"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("feat: add thing"), "{stdout}");
}

#[test]
fn the_check_needs_no_identity() {
    let repository = Repository::without_identity("identity-check");

    let output = repository.run_bare(&["-s", "feat: add thing", "--check-message"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_missing_directory_fails() {
    let repository = Repository::bare("missing");

    let output = Command::new(BINARY)
        .arg("-C")
        .arg("/nonexistent/ccommit")
        .env("XDG_CONFIG_HOME", &repository.config_home)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .expect("the binary runs");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "Failure: Directory `/nonexistent/ccommit` does not exist"
    );
}

#[test]
fn the_config_menu_writes_the_file_at_once() {
    let repository = Repository::clean("menu");
    let path = repository.config_home.join("ccommit/config.json");

    // Two rows down reaches `commit_on_finish`, and Enter toggles it.
    let outcome = repository.run(&["-m"], &[DOWN, DOWN, ENTER, ESC]);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let text = std::fs::read_to_string(&path).expect("the config file exists");
    assert!(text.contains("\"commit_on_finish\": false"), "{text}");
}

#[test]
fn the_config_menu_keeps_the_comments_in_the_file() {
    let repository = Repository::clean("menu-comments");
    let path = repository.config_home.join("ccommit/config.json");
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory is created");
    std::fs::write(
        &path,
        "{\n  // I want the gitmoji codes\n  \"real_emoji\": false\n}\n",
    )
    .expect("the config file is written");

    // Four rows down reaches `real_emoji`.
    let outcome = repository.run(&["-m"], &[DOWN, DOWN, DOWN, DOWN, ENTER, ESC]);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let text = std::fs::read_to_string(&path).expect("the config file exists");
    assert!(text.contains("// I want the gitmoji codes"), "{text}");
    assert!(text.contains("\"real_emoji\": true"), "{text}");
}

#[test]
fn the_config_menu_shows_the_file_and_not_the_command_line() {
    let repository = Repository::clean("menu-cli-show");
    repository.write_config("{\n  \"real_emoji\": false\n}\n");

    let outcome = repository.run(
        &[
            "-m",
            "--real-emoji=on",
            "--commit-types",
            "/nonexistent/types.json",
        ],
        &[ESC],
    );

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert!(
        outcome.text.contains("real_emoji: false"),
        "{}",
        outcome.text
    );
    assert!(
        !outcome.text.contains("real_emoji: true"),
        "{}",
        outcome.text
    );
    assert!(
        outcome.text.contains("commit_types: (embedded)"),
        "{}",
        outcome.text
    );
    assert!(
        !outcome.text.contains("/nonexistent/types.json"),
        "{}",
        outcome.text
    );
}

/// The command line says `off`, so a toggle over the flag value gives `true`.
/// Every key the menu can write, in the order the file receives them.
const KEYS: [&str; 16] = [
    "commit_types",
    "gitmojis",
    "commit_on_finish",
    "preview_on_finish",
    "real_emoji",
    "scope_step",
    "gitmoji_step",
    "long_description_step",
    "breaking_change_step",
    "deny_cli_long_message",
    "short_message_limit",
    "accent_color",
    "muted_color",
    "success_color",
    "warning_color",
    "error_color",
];

fn assert_holds_every_key(text: &str) {
    for key in KEYS {
        assert!(
            text.contains(&format!("\"{key}\"")),
            "{key} is absent:\n{text}"
        );
    }
}

#[test]
fn one_change_in_the_menu_writes_the_whole_config() {
    let repository = Repository::clean("menu-whole");
    let path = repository.config_home.join("ccommit/config.json");

    // Two rows down reaches `commit_on_finish`, and Enter toggles it.
    let outcome = repository.run(&["-m"], &[DOWN, DOWN, ENTER, ESC]);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let text = std::fs::read_to_string(&path).expect("the config file exists");
    assert_holds_every_key(&text);
    assert!(text.contains("\"commit_on_finish\": false"), "{text}");
    assert!(text.contains("\"real_emoji\": true"), "{text}");
    assert!(text.contains("\"accent_color\": null"), "{text}");
}

/// The file of a person keeps its comments, and the menu adds the keys it does not hold.
#[test]
fn the_menu_completes_a_file_that_a_person_wrote_by_hand() {
    let repository = Repository::clean("menu-complete");
    let path =
        repository.write_config("{\n  // I want the gitmoji codes\n  \"real_emoji\": false\n}\n");

    // Five rows down reaches `scope_step`, and Enter toggles it.
    let outcome = repository.run(&["-m"], &to_row(5, &[ESC]));

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let text = std::fs::read_to_string(&path).expect("the config file exists");
    assert_holds_every_key(&text);
    assert!(text.contains("// I want the gitmoji codes"), "{text}");
    assert!(text.contains("\"real_emoji\": false"), "{text}");
    assert!(text.contains("\"scope_step\": false"), "{text}");
    assert!(text.contains("\"gitmoji_step\": true"), "{text}");
}

/// The whole file the menu writes says nothing more than the defaults, so the run does not change.
#[test]
fn a_whole_config_file_of_defaults_runs_like_no_file_at_all() {
    let repository = Repository::with_history("cfg-whole");
    let menu = repository.run(&["-m"], &[DOWN, DOWN, ENTER, ENTER, ESC]);
    assert_eq!(menu.code, 0, "{}", menu.text);

    let text = std::fs::read_to_string(repository.config_home.join("ccommit/config.json"))
        .expect("the config file exists");
    assert_holds_every_key(&text);
    assert!(text.contains("\"commit_on_finish\": true"), "{text}");

    let mut keys = wizard_keys();
    keys.extend([b"the body".as_slice(), ENTER, ENTER, ENTER]);
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: ✨ add thing\n\nthe body");
}

/// A person can erase keys, so a file with one key alone still runs on the defaults.
#[test]
fn a_config_file_with_one_key_alone_runs_on_the_defaults() {
    let repository = Repository::with_history("cfg-one-key");
    repository.write_config("{\n  \"breaking_change_step\": false\n}\n");

    // The wizard holds the type, the scope, the gitmoji, the two descriptions, and the preview.
    let mut keys = wizard_keys();
    keys.extend([b"the body".as_slice(), ENTER, ENTER]);
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: ✨ add thing\n\nthe body");
}

#[test]
fn the_config_menu_shows_the_value_in_use_for_an_absent_key() {
    let repository = Repository::clean("menu-defaults");

    let outcome = repository.run(&["-m"], &[ESC]);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    for row in [
        "short_message_limit: 72 (default)",
        "accent_color: bright blue (default)",
        "muted_color: gray (default)",
        "success_color: bright green (default)",
        "warning_color: bright yellow (default)",
        "error_color: bright red (default)",
    ] {
        assert!(
            outcome.text.contains(row),
            "{row} is absent:\n{}",
            outcome.text
        );
    }
}

/// A value the file holds shows on its own, with no mark of the default.
#[test]
fn the_config_menu_shows_the_file_value_without_the_default_mark() {
    let repository = Repository::clean("menu-set");
    repository
        .write_config("{\n  \"accent_color\": \"#1e88e5\",\n  \"short_message_limit\": 80\n}\n");

    let outcome = repository.run(&["-m"], &[ESC]);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert!(
        outcome.text.contains("accent_color: #1e88e5"),
        "{}",
        outcome.text
    );
    assert!(
        outcome.text.contains("short_message_limit: 80"),
        "{}",
        outcome.text
    );
    assert!(!outcome.text.contains("80 (default)"), "{}", outcome.text);
}

#[test]
fn the_config_menu_toggles_the_file_value_and_not_the_command_line_one() {
    let repository = Repository::clean("menu-cli-toggle");
    let path = repository.write_config("{\n  \"commit_on_finish\": true\n}\n");

    // Two rows down reaches `commit_on_finish`, and Enter toggles it.
    let outcome = repository.run(&["-m", "--commit-on-finish=off"], &[DOWN, DOWN, ENTER, ESC]);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let text = std::fs::read_to_string(&path).expect("the config file exists");
    assert!(text.contains("\"commit_on_finish\": false"), "{text}");
}

/// The truecolor escape sequence that `#1e88e5` gives.
const BLUE_SEQUENCE: &str = "38;2;30;136;229";

/// The keys that walk `count` rows down the menu, then open the row.
fn to_row(count: usize, rest: &[&'static [u8]]) -> Vec<&'static [u8]> {
    let mut keys = vec![DOWN; count];
    keys.push(ENTER);
    keys.extend_from_slice(rest);
    keys
}

/// Ten rows down reaches `short_message_limit`, and Enter opens the text input.
fn limit_keys(rest: &[&'static [u8]]) -> Vec<&'static [u8]> {
    to_row(10, rest)
}

/// Eleven rows down reaches `accent_color`, and Enter opens the text input.
fn color_keys(rest: &[&'static [u8]]) -> Vec<&'static [u8]> {
    to_row(11, rest)
}

#[test]
fn the_config_menu_writes_the_limit() {
    let repository = Repository::clean("menu-limit");
    let path = repository.config_home.join("ccommit/config.json");

    let outcome = repository.run(&["-m"], &limit_keys(&[b"80", ENTER, ESC]));

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let text = std::fs::read_to_string(&path).expect("the config file exists");
    assert!(text.contains("\"short_message_limit\": 80"), "{text}");
}

#[test]
fn the_config_menu_writes_no_limit() {
    let repository = Repository::clean("menu-limit-none");
    let path = repository.config_home.join("ccommit/config.json");

    let outcome = repository.run(&["-m"], &limit_keys(&[b"none", ENTER, ESC]));

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let text = std::fs::read_to_string(&path).expect("the config file exists");
    assert!(text.contains("\"short_message_limit\": \"none\""), "{text}");
}

#[test]
fn the_config_menu_refuses_a_limit_under_the_minimum() {
    let repository = Repository::clean("menu-limit-bad");
    let path = repository.config_home.join("ccommit/config.json");

    let outcome = repository.run(&["-m"], &limit_keys(&[b"49", ENTER, ESC, ESC]));

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert!(outcome.text.contains("minimum of 50"), "{}", outcome.text);
    assert!(!path.exists(), "the refused limit reached the file");
}

#[test]
fn an_empty_limit_in_the_menu_gives_the_default_back() {
    let repository = Repository::clean("menu-limit-clear");
    let path = repository.write_config("{\n  \"short_message_limit\": 80\n}\n");

    let outcome = repository.run(&["-m"], &limit_keys(&[CTRL_U, ENTER, ESC]));

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let text = std::fs::read_to_string(&path).expect("the config file exists");
    assert!(text.contains("\"short_message_limit\": null"), "{text}");
}

#[test]
fn a_limit_from_the_config_file_cuts_the_first_line() {
    let repository = Repository::with_history("cfg-limit");
    repository.write_config("{\n  \"short_message_limit\": 50\n}\n");

    let keys: Vec<&[u8]> = vec![
        b"feat",
        ENTER,
        ENTER,
        b":sparkles:",
        ENTER,
        b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        ENTER,
        ENTER,
        ENTER,
        ENTER,
    ];
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let message = repository.last_message();
    assert_eq!(message.chars().count(), 50, "{message}");
    assert_eq!(message, format!("feat: ✨ {}", "x".repeat(42)));
}

#[test]
fn no_limit_from_the_command_line_keeps_the_whole_first_line() {
    let repository = Repository::with_history("cli-limit-none");

    let keys: Vec<&[u8]> = vec![
        b"feat",
        ENTER,
        ENTER,
        b":sparkles:",
        ENTER,
        b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        ENTER,
        ENTER,
        ENTER,
        ENTER,
    ];
    let outcome = repository.run(&["--short-message-limit=none"], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let message = repository.last_message();
    assert_eq!(message.chars().count(), 88, "{message}");
    assert_eq!(message, format!("feat: ✨ {}", "x".repeat(80)));
}

#[test]
fn the_command_line_beats_the_config_file_for_the_limit() {
    let repository = Repository::with_history("cli-limit");
    repository.write_config("{\n  \"short_message_limit\": \"none\"\n}\n");

    let keys: Vec<&[u8]> = vec![
        b"feat",
        ENTER,
        ENTER,
        b":sparkles:",
        ENTER,
        b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        ENTER,
        ENTER,
        ENTER,
        ENTER,
    ];
    let outcome = repository.run(&["--short-message-limit=60"], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message().chars().count(), 60);
}

#[test]
fn a_limit_flag_under_the_minimum_fails() {
    let repository = Repository::with_history("cli-limit-bad");

    let output = repository.run_bare(&["--short-message-limit", "49"]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("minimum of 50"), "{stderr}");
}

#[test]
fn a_limit_in_the_config_file_under_the_minimum_fails_with_the_path() {
    let repository = Repository::with_history("cfg-limit-bad");
    let path = repository.write_config("{\n  \"short_message_limit\": 49\n}\n");

    let output = repository.run_bare(&[]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("50"), "{stderr}");
    assert!(
        stderr.contains(&path.display().to_string()),
        "{stderr} does not name {}",
        path.display()
    );
}

#[test]
fn the_config_menu_writes_a_color() {
    let repository = Repository::clean("menu-color");
    let path = repository.config_home.join("ccommit/config.json");

    let outcome = repository.run(&["-m"], &color_keys(&[b"#1e88e5", ENTER, ESC]));

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let text = std::fs::read_to_string(&path).expect("the config file exists");
    assert!(text.contains("\"accent_color\": \"#1e88e5\""), "{text}");

    // The menu repaints itself with the color it just wrote.
    assert!(outcome.text.contains(BLUE_SEQUENCE), "{}", outcome.text);
}

#[test]
fn the_config_menu_takes_the_short_form_of_a_color() {
    let repository = Repository::clean("menu-color-short");
    let path = repository.config_home.join("ccommit/config.json");

    let outcome = repository.run(&["-m"], &color_keys(&[b"f0a", ENTER, ESC]));

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let text = std::fs::read_to_string(&path).expect("the config file exists");
    assert!(text.contains("\"accent_color\": \"#ff00aa\""), "{text}");
}

#[test]
fn the_config_menu_refuses_a_color_that_is_not_hex() {
    let repository = Repository::clean("menu-color-bad");
    let path = repository.config_home.join("ccommit/config.json");

    let outcome = repository.run(&["-m"], &color_keys(&[b"blue", ENTER, ESC, ESC]));

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert!(outcome.text.contains("not a hex color"), "{}", outcome.text);
    assert!(!path.exists(), "the refused color reached the file");
}

#[test]
fn an_empty_color_in_the_menu_gives_the_default_back() {
    let repository = Repository::clean("menu-color-clear");
    let path = repository.write_config("{\n  \"accent_color\": \"#1e88e5\"\n}\n");

    // Ctrl+U erases the value the input opens with.
    let outcome = repository.run(&["-m"], &color_keys(&[CTRL_U, ENTER, ESC]));

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let text = std::fs::read_to_string(&path).expect("the config file exists");
    assert!(text.contains("\"accent_color\": null"), "{text}");
}

#[test]
fn a_color_from_the_config_file_reaches_the_screen() {
    let repository = Repository::with_history("cfg-color");
    repository.write_config("{\n  \"accent_color\": \"#1e88e5\"\n}\n");

    let outcome = repository.run(&[], &[ESC]);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert!(outcome.text.contains(BLUE_SEQUENCE), "{}", outcome.text);
}

#[test]
fn a_color_from_the_command_line_beats_the_config_file() {
    let repository = Repository::with_history("cli-color");
    repository.write_config("{\n  \"accent_color\": \"#000000\"\n}\n");

    let outcome = repository.run(&["--accent-color=#1e88e5"], &[ESC]);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert!(outcome.text.contains(BLUE_SEQUENCE), "{}", outcome.text);
    assert!(!outcome.text.contains("38;2;0;0;0"), "{}", outcome.text);
}

#[test]
fn a_color_flag_that_is_not_hex_fails() {
    let repository = Repository::with_history("cli-color-bad");

    let output = repository.run_bare(&["--accent-color", "blue"]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not a hex color"), "{stderr}");
}

#[test]
fn a_color_in_the_config_file_that_is_not_hex_fails_with_the_path() {
    let repository = Repository::with_history("cfg-color-bad");
    let path = repository.write_config("{\n  \"accent_color\": \"blue\"\n}\n");

    let output = repository.run_bare(&[]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not a hex color"), "{stderr}");
    assert!(
        stderr.contains(&path.display().to_string()),
        "{stderr} does not name {}",
        path.display()
    );
}

#[test]
fn the_config_menu_needs_no_repository() {
    let repository = Repository::bare("menu-nogit");

    let outcome = repository.run(&["-m"], &[ESC]);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
}

#[test]
fn the_config_file_changes_the_run() {
    let repository = Repository::with_history("cfg-read");
    repository.write_config("{\n  \"real_emoji\": false,\n  \"preview_on_finish\": false\n}\n");

    let mut keys = wizard_keys();
    keys.extend([ENTER, ENTER]);
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: :sparkles: add thing");
}

#[test]
fn the_command_line_beats_the_config_file() {
    let repository = Repository::with_history("cfg-override");
    repository.write_config("{\n  \"real_emoji\": false\n}\n");

    let mut keys = wizard_keys();
    keys.extend([ENTER, ENTER, ENTER]);
    let outcome = repository.run(&["--real-emoji=on"], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: ✨ add thing");
}

/// The pair of the config menu test above: outside the menu, the flag wins over the same file.
#[test]
fn the_command_line_stops_a_commit_that_the_config_file_asks_for() {
    let repository = Repository::with_history("cfg-commit-off");
    repository.write_config("{\n  \"commit_on_finish\": true\n}\n");

    let mut keys = wizard_keys();
    keys.extend([ENTER, ENTER, ENTER]);
    let outcome = repository.run(&["--commit-on-finish=off"], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert!(
        outcome.text.contains("feat: ✨ add thing"),
        "{}",
        outcome.text
    );
    assert_eq!(repository.commit_count(), 1, "no commit was added");
}

#[test]
fn a_config_file_with_comments_still_loads() {
    let repository = Repository::with_history("cfg-comments");
    repository.write_config(
        "{\n  // no pictures for me\n  \"real_emoji\": false,\n  \"preview_on_finish\": false,\n}\n",
    );

    let mut keys = wizard_keys();
    keys.extend([ENTER, ENTER]);
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: :sparkles: add thing");
}

#[test]
fn a_malformed_config_file_fails_with_the_path() {
    let repository = Repository::with_history("cfg-broken");
    let path = repository.write_config("{ this is not json");

    let output = repository.run_bare(&[]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("Failure: Could not parse"), "{stderr}");
    assert!(
        stderr.contains(&path.display().to_string()),
        "{stderr} does not name {}",
        path.display()
    );
}

#[test]
fn a_missing_config_from_the_command_line_fails() {
    let repository = Repository::with_history("cfg-absent");

    let output = repository.run_bare(&["--config", "/nonexistent/ccommit.json"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "Failure: No config at `/nonexistent/ccommit.json`"
    );
}

#[test]
fn a_custom_commit_type_file_replaces_the_embedded_one() {
    let repository = Repository::with_history("custom-types");
    let types = repository.write(
        "types.json",
        "[{ \"name\": \"spike\", \"long\": \"Spike\", \"description\": \"A throwaway try\" }]\n",
    );

    // One entry, so Enter takes it with no filter.
    let keys: Vec<&[u8]> = vec![
        ENTER,
        ENTER,
        b":sparkles:",
        ENTER,
        b"add thing",
        ENTER,
        ENTER,
        ENTER,
        ENTER,
    ];
    let outcome = repository.run(&["--commit-types", types.to_str().expect("a path")], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "spike: ✨ add thing");
}

#[test]
fn a_custom_gitmoji_file_replaces_the_embedded_one() {
    let repository = Repository::with_history("custom-gitmojis");
    let gitmojis = repository.write(
        "gitmojis.json",
        "[{ \"name\": \":rocket:\", \"long\": \"\", \"description\": \"Ship it\", \"emoji\": \"🚀\" }]\n",
    );

    let keys: Vec<&[u8]> = vec![
        b"feat",
        ENTER,
        ENTER,
        ENTER,
        b"add thing",
        ENTER,
        ENTER,
        ENTER,
        ENTER,
    ];
    let outcome = repository.run(&["--gitmojis", gitmojis.to_str().expect("a path")], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: 🚀 add thing");
}

#[test]
fn a_malformed_data_file_fails_with_the_path() {
    let repository = Repository::with_history("bad-types");
    let types = repository.write("types.json", "{ \"not\": \"an array\" }");

    let output = repository.run_bare(&["--commit-types", types.to_str().expect("a path")]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("Failure: Could not parse"), "{stderr}");
    assert!(stderr.contains("types.json"), "{stderr}");
}

#[test]
fn an_empty_data_file_fails() {
    let repository = Repository::with_history("empty-types");
    let types = repository.write("types.json", "[]");

    let output = repository.run_bare(&["--commit-types", types.to_str().expect("a path")]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "Failure: No entries available for the commit type menu"
    );
}

#[test]
fn a_scope_from_the_settings_file_reaches_the_message() {
    let repository = Repository::with_history("scope-read");
    repository.write(
        ".vscode/settings.json",
        "{\n  // the scopes we use\n  \"conventionalCommits.scopes\": [\n    \"ui\",\n    \"api\",\n  ],\n  \"editor.tabSize\": 2\n}\n",
    );

    // The first row is `ui`, because the settings scopes come before the three fixed rows.
    let keys: Vec<&[u8]> = vec![
        b"feat",
        ENTER,
        ENTER,
        b":sparkles:",
        ENTER,
        b"add thing",
        ENTER,
        ENTER,
        ENTER,
        ENTER,
    ];
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat(ui): ✨ add thing");
}

#[test]
fn a_new_scope_keeps_the_comments_in_the_settings_file() {
    let repository = Repository::with_history("scope-write");
    let settings = repository.write(
        ".vscode/settings.json",
        "{\n  // the scopes we use\n  \"conventionalCommits.scopes\": [\n    \"ui\"\n  ]\n}\n",
    );

    // Two rows down from `ui` reaches "New scope".
    let keys: Vec<&[u8]> = vec![
        b"feat",
        ENTER,
        DOWN,
        DOWN,
        ENTER,
        b"cli",
        ENTER,
        b":sparkles:",
        ENTER,
        b"add thing",
        ENTER,
        ENTER,
        ENTER,
        ENTER,
    ];
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat(cli): ✨ add thing");

    let text = std::fs::read_to_string(&settings).expect("the settings file reads");
    assert!(text.contains("// the scopes we use"), "{text}");
    assert!(text.contains("\"ui\""), "{text}");
    assert!(text.contains("\"cli\""), "{text}");
}

#[test]
fn the_commit_msg_hook_sees_the_whole_message() {
    let repository = Repository::with_history("commit-msg");
    repository.add_hook(
        "commit-msg",
        "#!/bin/sh\ngrep -q '^feat: ' \"$1\" || exit 1\ngrep -q 'the body' \"$1\" || exit 1\n",
    );

    let mut keys = wizard_keys();
    keys.extend([b"the body".as_slice(), ENTER, ENTER, ENTER]);
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: ✨ add thing\n\nthe body");
}

#[test]
fn esc_from_a_later_step_returns_and_keeps_the_text() {
    let repository = Repository::with_history("back");

    // Type the description, walk back to the type menu, then walk forward again.
    let keys: Vec<&[u8]> = vec![
        b"feat",
        ENTER,
        ENTER,
        b":sparkles:",
        ENTER,
        b"add thing",
        ESC,
        ESC,
        ESC,
        b"fix",
        ENTER,
        ENTER,
        b":bug:",
        ENTER,
        ENTER,
        ENTER,
        ENTER,
        ENTER,
    ];
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "fix: 🐛 add thing");
}

#[test]
fn a_breaking_change_reaches_the_body() {
    let repository = Repository::with_history("breaking");

    let mut keys = wizard_keys();
    keys.extend([
        b"the body".as_slice(),
        ENTER,
        b"BREAKING CHANGE: the API moved",
        ENTER,
        ENTER,
    ]);
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(
        repository.last_message(),
        "feat: ✨ add thing\n\nthe body\nBREAKING CHANGE: the API moved"
    );
}

#[test]
fn a_body_over_several_lines_reaches_the_commit() {
    let repository = Repository::with_history("many-lines");

    let mut keys = wizard_keys();
    keys.extend([
        b"first line".as_slice(),
        SHIFT_ENTER,
        b"second line",
        OPTION_ENTER,
        b"third line",
        ENTER,
        ENTER,
        ENTER,
    ]);
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(
        repository.last_message(),
        "feat: ✨ add thing\n\nfirst line\nsecond line\nthird line"
    );
}

#[test]
fn ctrl_c_quits_with_no_commit() {
    let repository = Repository::with_history("ctrl-c");

    let outcome = repository.run(&[], &[b"feat", ENTER, ENTER, CTRL_C]);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.commit_count(), 1);
}

#[test]
fn an_empty_short_description_does_not_move_on() {
    let repository = Repository::with_history("refuse-empty");

    // The Enter after the gitmoji leaves the wizard on the short description, and the bare Enter
    // that follows changes nothing. The text then submits it.
    let keys: Vec<&[u8]> = vec![
        b"feat",
        ENTER,
        ENTER,
        b":sparkles:",
        ENTER,
        ENTER,
        ENTER,
        b"add thing",
        ENTER,
        ENTER,
        ENTER,
        ENTER,
    ];
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: ✨ add thing");
}

#[test]
fn the_preview_step_disappears_when_the_config_turns_it_off() {
    let repository = Repository::with_history("no-preview-cfg");
    repository.write_config("{\n  \"preview_on_finish\": false\n}\n");

    let mut keys = wizard_keys();
    keys.extend([ENTER, ENTER]);
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: ✨ add thing");
}

#[test]
fn the_step_flags_cut_the_wizard_down_to_two_steps() {
    let repository = Repository::with_history("no-steps-cli");

    // The type menu, then the short description, then the preview.
    let keys: Vec<&[u8]> = vec![b"feat", ENTER, b"add thing", ENTER, ENTER];
    let outcome = repository.run(
        &[
            "--scope-step=off",
            "--gitmoji-step=off",
            "--long-description-step=off",
            "--breaking-change-step=off",
        ],
        &keys,
    );

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: add thing");
}

#[test]
fn the_breaking_change_step_disappears_when_the_config_turns_it_off() {
    let repository = Repository::with_history("no-breaking-cfg");
    repository.write_config("{\n  \"breaking_change_step\": false\n}\n");

    let mut keys = wizard_keys();
    keys.extend([b"the body".as_slice(), ENTER, ENTER]);
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: ✨ add thing\n\nthe body");
}

#[test]
fn the_config_menu_writes_a_step_key() {
    let repository = Repository::clean("menu-step");
    let path = repository.config_home.join("ccommit/config.json");

    // Eight rows down reaches `breaking_change_step`, and Enter toggles it.
    let outcome = repository.run(&["-m"], &to_row(8, &[ESC]));

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let text = std::fs::read_to_string(&path).expect("the config file exists");
    assert!(text.contains("\"breaking_change_step\": false"), "{text}");
}

#[test]
fn the_short_description_stops_at_the_limit() {
    let repository = Repository::with_history("limit");

    // 80 characters typed into a limit of 64.
    let keys: Vec<&[u8]> = vec![
        b"feat",
        ENTER,
        ENTER,
        b":sparkles:",
        ENTER,
        b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        ENTER,
        ENTER,
        ENTER,
        ENTER,
    ];
    let outcome = repository.run(&[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let message = repository.last_message();
    assert_eq!(message.chars().count(), 72, "{message}");
    assert_eq!(message, format!("feat: ✨ {}", "x".repeat(64)));
}

#[test]
fn a_short_message_flag_commits_without_the_wizard() {
    let repository = Repository::with_history("direct-commit");

    let output = repository.run_bare(&["-s", "feat(ui): ✨ add thing", "-l", "the body"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        repository.last_message(),
        "feat(ui): ✨ add thing\n\nthe body"
    );
}

#[test]
fn a_long_message_of_several_lines_reaches_the_commit() {
    let repository = Repository::with_history("direct-lines");

    let output = repository.run_bare(&["-s", "feat: add thing", "-l", "first line\nsecond line"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        repository.last_message(),
        "feat: add thing\n\nfirst line\nsecond line"
    );
}

#[test]
fn a_gitmoji_code_passes_the_direct_validation() {
    let repository = Repository::with_history("direct-code");

    let output = repository.run_bare(&["-s", "fix: :bug: squash it"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(repository.last_message(), "fix: :bug: squash it");
}

#[test]
fn an_unknown_type_stops_the_direct_commit() {
    let repository = Repository::with_history("direct-bad-type");

    let output = repository.run_bare(&["-s", "wip: add thing"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not one of the commit types"), "{stderr}");
    assert_eq!(repository.commit_count(), 1, "no commit was added");
}

#[test]
fn a_first_line_over_the_limit_stops_the_direct_commit() {
    let repository = Repository::with_history("direct-limit");
    let long_line = format!("feat: {}", "x".repeat(80));

    let output = repository.run_bare(&["-s", &long_line]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("over the limit of 72"), "{stderr}");

    let output = repository.run_bare(&["-s", &long_line, "--short-message-limit=none"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(repository.last_message(), long_line);
}

#[test]
fn the_direct_path_prints_when_the_commit_is_off() {
    let repository = Repository::with_history("direct-print");

    let output = repository.run_bare(&["-s", "feat: add thing", "--commit-on-finish=off"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("feat: add thing"), "{stdout}");
    assert_eq!(repository.commit_count(), 1, "no commit was added");
}

#[test]
fn a_direct_commit_without_staged_changes_fails() {
    let repository = Repository::clean("direct-clean");

    let output = repository.run_bare(&["-s", "feat: add thing"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "Failure: Can't commit without changes added"
    );
}

#[test]
fn the_config_file_denies_the_long_message_flag() {
    let repository = Repository::with_history("deny-long-cfg");
    repository.write_config("{\n  \"deny_cli_long_message\": true\n}\n");

    let output = repository.run_bare(&["-s", "feat: add thing", "-l", "the body"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("denies `--long-message`"), "{stderr}");
    assert_eq!(repository.commit_count(), 1, "no commit was added");
}

#[test]
fn the_deny_flag_stops_the_long_message() {
    let repository = Repository::with_history("deny-long-cli");

    let output = repository.run_bare(&[
        "-s",
        "feat: add thing",
        "-l",
        "the body",
        "--deny-cli-long-message=on",
    ]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("denies `--long-message`"), "{stderr}");
    assert_eq!(repository.commit_count(), 1, "no commit was added");
}

#[test]
fn the_deny_flag_beats_the_config_file() {
    let repository = Repository::with_history("deny-long-override");
    repository.write_config("{\n  \"deny_cli_long_message\": true\n}\n");

    let output = repository.run_bare(&[
        "-s",
        "feat: add thing",
        "-l",
        "the body",
        "--deny-cli-long-message=off",
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(repository.last_message(), "feat: add thing\n\nthe body");
}

#[test]
fn the_deny_key_leaves_the_short_message_alone() {
    let repository = Repository::with_history("deny-long-short-only");
    repository.write_config("{\n  \"deny_cli_long_message\": true\n}\n");

    let output = repository.run_bare(&["-s", "feat: add thing"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(repository.last_message(), "feat: add thing");
}

#[test]
fn the_config_menu_writes_the_deny_key() {
    let repository = Repository::clean("menu-deny");
    let path = repository.config_home.join("ccommit/config.json");

    // Nine rows down reaches `deny_cli_long_message`, and Enter toggles it.
    let outcome = repository.run(&["-m"], &to_row(9, &[ESC]));

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let text = std::fs::read_to_string(&path).expect("the config file exists");
    assert!(text.contains("\"deny_cli_long_message\": true"), "{text}");
}

#[test]
fn the_check_leaves_the_repository_alone() {
    let repository = Repository::with_history("check");

    let output = repository.run_bare(&["-s", "feat: ✨ add thing", "--check-message"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "The message is valid"
    );
    assert_eq!(repository.commit_count(), 1, "no commit was added");
}

#[test]
fn the_check_needs_no_repository() {
    let repository = Repository::bare("check-nogit");

    let output = repository.run_bare(&["-s", "feat: add thing", "--check-message"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn the_check_fails_on_an_invalid_message() {
    let repository = Repository::bare("check-bad");

    let output = repository.run_bare(&["-s", "feat:no space", "--check-message"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No space follows"), "{stderr}");
}

/// The amend needs no staged change, so a repository with a clean tree still reaches the wizard.
#[test]
fn the_wizard_amends_the_last_commit() {
    let repository = Repository::clean("amend-wizard");

    let mut keys = wizard_keys();
    keys.extend([ENTER, ENTER, ENTER]);
    let outcome = repository.run(&["--amend"], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(repository.last_message(), "feat: ✨ add thing");
    assert_eq!(repository.commit_count(), 1, "{}", outcome.text);
}

#[test]
fn the_preview_footer_names_the_amend() {
    let repository = Repository::with_history("amend-footer");

    // Ctrl+C on the preview leaves the last commit alone.
    let mut keys = wizard_keys();
    keys.extend([ENTER, ENTER, CTRL_C]);
    let outcome = repository.run(&["--amend"], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert!(outcome.text.contains("↵ amend"), "{}", outcome.text);
    assert_eq!(repository.last_message(), "chore: first");
}

/// The staged file joins the commit that the amend replaces.
#[test]
fn a_direct_amend_replaces_the_last_commit() {
    let repository = Repository::with_history("amend-direct");

    let output = repository.run_bare(&["--amend", "-s", "feat: ✨ add thing", "-l", "the body"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(repository.last_message(), "feat: ✨ add thing\n\nthe body");
    assert_eq!(repository.commit_count(), 1);
    assert_eq!(repository.files_in_head(), ["one.txt", "readme.txt"]);
}

#[test]
fn an_amend_with_nothing_staged_changes_the_message_alone() {
    let repository = Repository::clean("amend-message-only");

    let output = repository.run_bare(&["--amend", "-s", "fix: 🐛 squash it"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(repository.last_message(), "fix: 🐛 squash it");
    assert_eq!(repository.commit_count(), 1);
    assert_eq!(repository.files_in_head(), ["readme.txt"]);
}

#[test]
fn an_amend_in_a_repository_without_a_commit_fails() {
    let repository = Repository::unborn("amend-unborn");

    let output = repository.run_bare(&["--amend", "-s", "feat: add thing"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "Failure: Can't amend without a commit"
    );
    assert_eq!(repository.commit_count(), 0);
}

#[test]
fn an_amend_with_the_commit_off_fails() {
    let repository = Repository::with_history("amend-print");

    let output =
        repository.run_bare(&["--amend", "-s", "feat: add thing", "--commit-on-finish=off"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`--amend` needs the commit"), "{stderr}");
    assert_eq!(repository.last_message(), "chore: first");
}

#[test]
fn an_amend_with_the_check_fails_before_the_run() {
    let repository = Repository::with_history("amend-check");

    let output = repository.run_bare(&["--amend", "-s", "feat: add thing", "--check-message"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(repository.last_message(), "chore: first");
}

#[test]
fn a_failing_pre_commit_hook_stops_the_amend() {
    let repository = Repository::with_history("amend-hook");
    repository.add_failing_pre_commit_hook();

    let output = repository.run_bare(&["--amend", "-s", "feat: add thing"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("hook says no"), "{stderr}");
    assert_eq!(repository.last_message(), "chore: first");
}

#[test]
fn the_working_directory_flag_picks_the_repository() {
    let target = Repository::with_history("target");
    let elsewhere = Repository::with_history("elsewhere");

    // Run from `elsewhere`, but point `-C` at `target`.
    let mut keys = wizard_keys();
    keys.extend([ENTER, ENTER, ENTER]);
    let outcome = target.run_from(&elsewhere.path, &[], &keys);

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(target.last_message(), "feat: ✨ add thing");
    assert_eq!(elsewhere.last_message(), "chore: first");
}
