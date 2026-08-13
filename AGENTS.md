ccommit — conventional commits from the terminal. See `README.md` for usage.

## Shell

- **CRITICAL**: do not **EVER** run `git` commands, with these exceptions:
  - Read-only inspection is always allowed: `git diff` (any flags and paths) and `git status`.
  - `git add` and `git commit` are allowed **only** when the user asks for a commit (for example through `/commit`). Never autonomously. Never as a side effect of another task.
  - Everything else that changes the history, the index, or a remote stays forbidden. This covers checkout, reset, rebase, push, `branch -D`, and every form of `stash`.

## Output Style

The **Simple English** output style is the default for this repository. It holds the full ASD-STE100 rule catalog: `.claude/output-styles/simple-english.md`.

**ALWAYS** write in ASD-STE100 Simplified Technical English. This applies to every chat reply, code comment, docstring, document, and commit message:

- One idea per sentence. Maximum 20 words for instructions, 25 for explanations.
- Active voice and simple tenses. No present perfect. No `-ing` form as a verb.
- Approved modals: can, will, must. Banned: should, would, may, might, could.
- Condition before command: "If the build fails, read the log."
- One word per concept for the whole text. No synonym rotation.
- Delete filler: simply, seamlessly, robust, comprehensive, leverage, "it is worth noting".
- No semicolons. No contractions. Keep the articles and keep "that".
- Code, identifiers, paths, and quoted errors stay exact.

No lint rule and no test checks this style. It holds only when you apply it. Read the output style file for the rule numbers, the substitution tables, and the self-check.

**ALWAYS** write code and docs in English. This covers identifiers, comments, docstrings, tests, and documents. Only user-facing copy holds another language, and only when the user asks for it.

## Comments

- **Default to none**: prefer self-explanatory names and structure over comments. Do not restate the code. Do not add ceremony.
- **Be concise**: one short line where it genuinely adds something. Explain the **why**, that is the non-obvious intent or a subtle constraint. Do not explain the **what** that the code already states.
- **Write timeless comments**: a comment describes the code as it is **now**. Never narrate history, iterations, or transitions ("changed to X", "now resolves instead of", "previously we"). That is the job of `git`.
- **No region-dividing comments** unless the user asks for them.
- **Precedence**: these rules decide **whether** a comment exists. The Output Style decides **how** it reads.
