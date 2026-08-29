# ccommit

Conventional commits from the terminal.

`ccommit` asks for the type, the scope, the gitmoji, and the description. It then builds the message
and calls `git commit`. The commit types and the gitmojis are compiled into the binary, so the
program needs no data directory and no configuration file.

## Message from the command line

`-s` takes the first line from the command line and skips the wizard. `-l` adds the body under one
empty line, as `git commit -m -m` does. The program makes sure that the wizard can produce the same
first line:

- The type must be one of the commit types.
- The scope is optional and sits between `(` and `)`.
- The gitmoji is optional, as the code or as the emoji, and it must be one of the gitmojis.
- The short description must not be empty.
- The first line must stay inside `short_message_limit`.

A valid message commits at once, and `--commit-on-finish=off` prints it instead. `--check-message`
only validates the message and exits: code 0 for a valid message, code 1 with the reason for an
invalid one.

```sh
ccommit -s "feat(ui): ✨ add dark mode" -l "The palette follows the system setting."
ccommit -s "feat(ui): ✨ add dark mode" --check-message
```

## Configuration

Every key holds a matching flag. The file lives at `~/.config/ccommit/config.json`, or under
`XDG_CONFIG_HOME`. It can hold comments and trailing commas. Precedence is the command line, then
the file, then the default. `ccommit -m` opens a menu over the fifteen keys and writes the file for
you. The menu shows the values of the file alone. The flags change one run, so they stay out of the
menu.

Each change in the menu writes all fifteen keys, so the file stays complete. A key the file already
holds keeps its place, and the comments around it survive. `null` stands for the default, so a key
with `null` and an absent key mean the same. The program never reads the file as complete, and a key
you erase by hand gives the default again.

A step that is off never appears, and its value stays empty. The type step and the short description
step always appear.

| Flag                        | Config key              | Action                                                   | Accepted values              | Default                         |
| --------------------------- | ----------------------- | -------------------------------------------------------- | ---------------------------- | ------------------------------- |
| `-C`, `--working-directory` |                         | Set the working directory                                | a path                       | `.`                             |
| `-m`, `--config-menu`       |                         | Open the configuration menu and exit                     | none                         | off                             |
| `--config`                  |                         | Path to the configuration file                           | a path                       | `~/.config/ccommit/config.json` |
| `-s`, `--short-message`     |                         | Commit with this first line and skip the wizard          | text                         |                                 |
| `-l`, `--long-message`      |                         | Put this text in the body, under one empty line          | text                         |                                 |
| `--check-message`           |                         | Validate the short message and exit without a commit     | none                         | off                             |
| `--commit-types`            | `commit_types`          | Path to a JSON file with the commit types                | a path                       | embedded                        |
| `--gitmojis`                | `gitmojis`              | Path to a JSON file with the gitmojis                    | a path                       | embedded                        |
| `--commit-on-finish`        | `commit_on_finish`      | Commit when the wizard ends                              | `on`, `off`, `true`, `false` | on                              |
| `--preview-on-finish`       | `preview_on_finish`     | Show the message before the wizard ends                  | `on`, `off`, `true`, `false` | on                              |
| `--real-emoji`              | `real_emoji`            | Put the emoji in the message instead of the code         | `on`, `off`, `true`, `false` | on                              |
| `--scope-step`              | `scope_step`            | Show the scope step                                      | `on`, `off`, `true`, `false` | on                              |
| `--gitmoji-step`            | `gitmoji_step`          | Show the gitmoji step                                    | `on`, `off`, `true`, `false` | on                              |
| `--long-description-step`   | `long_description_step` | Show the long description step                           | `on`, `off`, `true`, `false` | on                              |
| `--breaking-change-step`    | `breaking_change_step`  | Show the breaking change step                            | `on`, `off`, `true`, `false` | on                              |
| `--short-message-limit`     | `short_message_limit`   | Characters allowed on the first line                     | 50 or more, or `none`        | 72                              |
| `--accent-color`            | `accent_color`          | Color of the titles, the prompts, and the cursor         | a hex color                  | bright blue                     |
| `--muted-color`             | `muted_color`           | Color of the key help and the file path                  | a hex color                  | gray                            |
| `--success-color`           | `success_color`         | Color of the menu filter and the short counter           | a hex color                  | bright green                    |
| `--warning-color`           | `warning_color`         | Color of the counter above half the limit                | a hex color                  | bright yellow                   |
| `--error-color`             | `error_color`           | Color of the failure line and the counter near the limit | a hex color                  | bright red                      |

## Install

```sh
pnpm build
```

The binary lands at `target/release/ccommit`. Copy it to a directory on your `PATH`.
