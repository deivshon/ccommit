use clap::Parser;

use ccommit::cli::Args;
use ccommit::config::{self, Partial};
use ccommit::screen::config_menu::ConfigMenu;
use ccommit::screen::wizard::Wizard;
use ccommit::{data, git, ui};

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

fn run_wizard(args: &Args) {
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
    if !git::has_staged_changes(working_directory) {
        failure("Can't commit without changes added");
    }

    let config = load_config(args);

    let commit_types = data::commit_types(config.commit_types.as_deref()).or_fail();
    let gitmojis = data::gitmojis(config.gitmojis.as_deref()).or_fail();
    if commit_types.is_empty() {
        failure("No entries available for the commit type menu");
    }
    if gitmojis.is_empty() {
        failure("No entries available for the gitmoji menu");
    }

    let commit_on_finish = config.commit_on_finish;
    let mut wizard = Wizard::new(config, working_directory.clone(), commit_types, gitmojis);

    // The terminal is already restored here, so output reaches the real screen.
    let Some(message) = ui::run(&mut wizard).or_fail() else {
        return;
    };

    if commit_on_finish {
        git::commit(working_directory, &message).or_fail();
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
