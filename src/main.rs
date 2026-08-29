use clap::Parser;

use ccommit::cli::Args;
use ccommit::config::{self, Partial};
use ccommit::git::MissingIdentity;
use ccommit::message::ConventionalCommit;
use ccommit::screen::config_menu::ConfigMenu;
use ccommit::screen::wizard::Wizard;
use ccommit::{data, git, ui, validate};

fn failure(message: &str) -> ! {
    eprintln!("Failure: {message}");
    std::process::exit(1);
}

trait OrFail<T> {
    fn or_fail(self) -> T;
}

impl<T> OrFail<T> for Result<T, String> {
    fn or_fail(self) -> T {
        self.unwrap_or_else(|error| failure(&error))
    }
}

fn main() {
    let args = Args::parse();

    // The configuration menu needs no repository, so it skips the three startup checks.
    if args.config_menu {
        run_config_menu(&args);
    } else if let Some(short_message) = args.short_message.clone() {
        run_direct(&args, &short_message);
    } else {
        run_wizard(&args);
    }
}

fn run_config_menu(args: &Args) {
    let path = match config::resolve_path(args.config.as_deref()).or_fail() {
        Some(path) => path,
        None => failure("Could not find the home directory"),
    };

    let file = config::load(&path).or_fail();
    let mut menu = ConfigMenu::new(path, config::from_file(file));
    ui::run(&mut menu).or_fail();
}

/// The three startup checks every commit shares.
fn require_repository(args: &Args) {
    let working_directory = &args.working_directory;
    if !working_directory.is_dir() {
        failure(&format!(
            "Directory `{}` does not exist",
            working_directory.display()
        ));
    }

    if !git::is_repository(working_directory) {
        failure("Can't commit outside of a git repository");
    }

    // The amend keeps the tree of the last commit, so it asks for that commit instead.
    if args.amend {
        if !git::head_exists(working_directory) {
            failure("Can't amend without a commit");
        }
    } else if !git::has_staged_changes(working_directory) {
        failure("Can't commit without changes added");
    }
}

/// The amend runs through `git commit`, so a run that only prints the message cannot do it.
fn require_commit_for_amend(args: &Args, config: &config::Config) {
    if args.amend && !config.commit_on_finish {
        failure("`--amend` needs the commit. Set `commit_on_finish` to true to allow it");
    }
}

/// Runs before the wizard, because `git commit` refuses an unknown identity.
fn require_identity(args: &Args) {
    let Some(missing) = git::missing_identity(&args.working_directory) else {
        return;
    };

    let name = "  git config user.name \"Your Name\"";
    let email = "  git config user.email \"you@example.com\"";
    let hint = match missing {
        MissingIdentity::Name => name.to_string(),
        MissingIdentity::Email => email.to_string(),
        MissingIdentity::Both => format!("{name}\n{email}"),
    };

    failure(&format!(
        "Can't commit without a git identity. Set it with:\n{hint}"
    ));
}

/// Builds the commit from the flags, without the wizard.
fn run_direct(args: &Args, short_message: &str) {
    // The check only validates, so it needs no repository.
    if !args.check_message {
        require_repository(args);
    }

    let config = load_config(args);
    if config.deny_cli_long_message && args.long_message.is_some() {
        failure(
            "The configuration denies `--long-message`. Set `deny_cli_long_message` to false to allow it",
        );
    }

    require_commit_for_amend(args, &config);
    if !args.check_message && config.commit_on_finish {
        require_identity(args);
    }

    let commit_types = data::commit_types(config.commit_types.as_deref()).or_fail();
    let gitmojis = data::gitmojis(config.gitmojis.as_deref()).or_fail();

    validate::validate(
        short_message,
        &commit_types,
        &gitmojis,
        config.message_limit(),
    )
    .or_fail();

    let message = ConventionalCommit {
        short_message: short_message.to_string(),
        long_message: args.long_message.clone().unwrap_or_default(),
    };

    if args.check_message {
        println!("The message is valid");
    } else if config.commit_on_finish {
        git::commit(&args.working_directory, &message, args.amend).or_fail();
    } else {
        println!("{}", message.text());
    }
}

fn run_wizard(args: &Args) {
    require_repository(args);
    let working_directory = &args.working_directory;

    let config = load_config(args);
    require_commit_for_amend(args, &config);
    if config.commit_on_finish {
        require_identity(args);
    }

    let commit_types = data::commit_types(config.commit_types.as_deref()).or_fail();
    let gitmojis = data::gitmojis(config.gitmojis.as_deref()).or_fail();
    if commit_types.is_empty() {
        failure("No entries available for the commit type menu");
    }
    if gitmojis.is_empty() {
        failure("No entries available for the gitmoji menu");
    }

    let commit_on_finish = config.commit_on_finish;
    let mut wizard = Wizard::new(config, working_directory.clone(), commit_types, gitmojis)
        .with_amend(args.amend);

    // The terminal is already restored here, so output reaches the real screen.
    let Some(message) = ui::run(&mut wizard).or_fail() else {
        return;
    };

    if commit_on_finish {
        git::commit(working_directory, &message, args.amend).or_fail();
    } else {
        println!("{}", message.text());
    }
}

fn load_config(args: &Args) -> config::Config {
    let file = match config::resolve_path(args.config.as_deref()).or_fail() {
        Some(path) => config::load(&path).or_fail(),
        None => Partial::default(),
    };

    config::merge(args.overrides(), file)
}
