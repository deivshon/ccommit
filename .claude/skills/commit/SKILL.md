---
name: commit
description: Commit the current changes with a conventional commit message in the house style - type, scope, gitmoji, imperative short message. Use when the user asks for a commit or when a task ends with one.
user-invocable: true
disable-model-invocation: false
---

# Commit

This skill defines the house rules for commit messages, and it directs you to
commit the current changes now. Do not stop at a message proposal. Write the
message, then run the commit.

## Format

```
type(scope): emoji description
```

- The short message is one line: the type, an optional scope, a colon, one
  space, one gitmoji, one space, and the description.
- The whole short message stays at 72 characters or fewer.
- A long message is almost never needed. See "Long message" below.

Example short messages:

```
feat(api-client): ✨ add the client compatibility namespace
fix(mobile): 🐛 use Apple's device file format for the ad-hoc profile
refactor(version-skew): ♻️ make every version gate variable optional
chore: 🔧 remove claude output style and related setting
test(be-e2e): ✅ cover the client version gate endpoint
```

## What, how, why

- The short message states the **what**: the concrete change, named precisely.
- The diff itself shows the **how**. Never spend short message characters on it.
- The long message holds the **why**, and only when the why is not trivial.
  Almost always, the why is plain from the short message or the diff, so almost
  no commit carries a long message.

## Type

Use exactly one of these lowercase types:

| Type       | Use for                                                                  |
| ---------- | ------------------------------------------------------------------------ |
| `feat`     | A new feature or a visible behavior change                               |
| `fix`      | A bug fix                                                                |
| `refactor` | A code change that neither fixes a bug nor adds a feature                |
| `perf`     | A code change that improves performance                                  |
| `test`     | New tests or corrections to existing tests                               |
| `docs`     | Documentation only changes, including code comments and AI documentation |
| `style`    | Formatting changes that do not change the meaning of the code            |
| `build`    | The build system, dependency pins, or version bumps                      |
| `ci`       | CI configuration files and scripts                                       |
| `chore`    | Other changes, for example tool configuration or migrations              |
| `revert`   | A revert of an earlier commit                                            |

Do not invent other types. `hotfix`, `feature`, and `wip` are defects.

When the diff does not fit one type or one gitmoji, first make sure that the
diff cannot split into more than one commit. A split is possible only when
each separate commit keeps these invariants:

- The project typechecks.
- The project has no lint errors.
- The project builds.

When a split is not possible, pick the type and the emoji that sit
semantically closest to the diff.

## Scope

- Read `conventionalCommits.scopes` in `.vscode/settings.json` first. When the
  list exists, pick a scope from it.
- When the list does not exist, omit the scope.
- Use `general` for a change across many scopes, when the list defines it or
  an analogous scope.

## Gitmoji

Put exactly one emoji after the colon. Write the real Unicode emoji, not the
`:code:` form. Pick the emoji from the intent, not from the type alone:

| Emoji | Intent                                 | Common type       |
| ----- | -------------------------------------- | ----------------- |
| ✨    | A new feature                          | `feat`            |
| 🐛    | A bug fix                              | `fix`             |
| 🩹    | A simple, non-critical fix             | `fix`             |
| 💄    | UI and style changes                   | `feat`, `fix`     |
| ♻️    | A refactor                             | `refactor`        |
| 🔥    | Removal of code or files               | `refactor`        |
| 🚚    | A move or a rename of files or modules | `refactor`        |
| 🏗️    | An architectural change                | `refactor`        |
| ⚡️    | A performance improvement              | `perf`            |
| ✅    | Tests                                  | `test`            |
| 📝    | Documentation                          | `docs`            |
| 💡    | Comments in the source code            | `docs`            |
| 🎨    | Format or structure of the code        | `style`           |
| 🔧    | Configuration files                    | `chore`           |
| 🔨    | Development scripts                    | `chore`           |
| 🗃️    | Database changes and migrations        | `chore`, `feat`   |
| ⬆️    | A dependency upgrade                   | `build`           |
| 📌    | A dependency pin                       | `build`           |
| ➖    | Removal of a dependency                | `build`           |
| 👷    | CI configuration                       | `ci`              |
| 💚    | A CI fix                               | `ci`              |
| 🚨    | Lint warnings and their fixes          | `fix`, `refactor` |
| 🦺    | Validation code                        | `feat`            |
| 🌐    | Internationalization and copy          | `fix`, `feat`     |
| ♿️    | Accessibility                          | `fix`             |
| 🔒️    | Security                               | `build`, `fix`    |
| ⏪️    | A revert                               | `revert`          |

## AI files

A file that an AI agent reads is not automatically a configuration file. Two
classes exist, and each one takes a different type and emoji.

| File                                                   | Class               | Type    | Emoji |
| ------------------------------------------------------ | ------------------- | ------- | ----- |
| `AGENTS.md`, `CLAUDE.md`, and other agent instructions | Documentation       | `docs`  | 📝    |
| Skill files, agent files, and output styles            | Development tooling | `chore` | 🔨    |

Agent instruction files hold prose for a reader, so they are documentation.
Skill files and agent files add a capability to the development environment, so
they are development tooling.

## Description

- Write the imperative present form: `add feature`, never `added feature` and
  never `adds feature`. The line completes the sentence "this commit will ...".
- Start with a lowercase letter. A proper noun or an identifier keeps its case.
- Do not end with a period.
- Lead with a verb. The house verbs: `add`, `fix`, `remove`, `use`, `update`,
  `bump`, `make`, `drop`, `rename`, `move`.
- Wrap identifiers, file names, flags, and rule names in backticks:
  ``chore: 🔧 enable `eslint/no-duplicate-imports` ``.
- Join two actions of one logical change with `+`:
  `feat: ✨ add config file handling + option to use actual emojis`.
- Name the concrete change, not the activity. Write
  `fix survey create breadcrumbs`, not `various fixes`.

## Long message

Default to none. Very, very few commits ever need one. Add a long message only
when the why of the change is real information that the short message and the
diff cannot show, for example a non-obvious constraint, a rejected alternative,
or an incompatibility that forced the change.

When one is needed:

- Explain the why, not a restatement of the diff.
- Use prose paragraphs, wrapped at about 80 characters. Use a bullet list only
  for enumerable items, for example a list of rules or of packages.
- Write in the present tense. Never write `this commit ...`, and never narrate
  the work history.

## Breaking changes

The `!` marker and the `BREAKING CHANGE:` footer exist purely for external
consumers: interfaces that live outside this repository. Examples: a published
package, an API with shipped mobile clients, a CLI that other people script
against. Release tooling parses the markers to bump the major version and to
build the changelog.

- When a change breaks an external consumer, append `!` after the type or the
  scope, and add a footer to the long message:
  `BREAKING CHANGE: <what breaks and what the consumer must do>`.
- In a monorepo that deploys the system as one unit, no external consumer
  exists, so the markers make no sense there. When the break is worth a note,
  describe it in plain words in the long message.

## Never

- Never write the past tense: `added`, `fixed`, `updated`.
- Never start the description with an uppercase letter.
- Never end the short message with a period.
- Never omit the emoji or use more than one.
- Never mix unrelated changes in one commit.
- Never write vague short messages: `wip`, `misc`, `updates`, `some fixes`.
- Never use `!` or a `BREAKING CHANGE:` footer when every consumer of the
  change lives inside the repository.
- Never add issue or PR numbers by hand. The bots add them.

## Procedure

1. Run `git status` and `git diff --cached`. If nothing is staged, stage the
   files that belong to this change, and nothing else.
2. Pick the type, the scope, and the emoji from the tables above.
3. Write the short message. Count the characters and stay at 72 or fewer.
4. In the rare case that the why needs recording, write the long message.
5. Commit now. When the `ccommit` binary is available, run
   `ccommit -s "<short message>"`, and add `-l "<long message>"` when one
   exists. `ccommit` validates the message on its own. When the binary is not
   available, run `git commit -m "<short message>"`, and pass the long message
   (if any) as a second `-m` value.
