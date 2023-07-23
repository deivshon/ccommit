import subprocess

from lib.builder import ConventionalCommit


def inside_repository(path: str) -> bool:
    p = subprocess.run(
        [
            "git",
            "-C",
            path,
            "rev-parse",
            "--is-inside-work-tree"
        ],
        capture_output=True
    )

    return p.returncode == 0


def commit(path: str, commit_message: ConventionalCommit):
    subprocess.run(
        [
            "git",
            "-C",
            path,
            "commit",
            "-m",
            commit_message.short_message,
            "-m",
            commit_message.long_message
        ]
    )


def changes_added(path: str) -> bool:
    p = subprocess.run(
        [
            "git",
            "-C",
            path,
            "diff-index",
            "--cached",
            "--quiet",
            "HEAD"
        ],
        capture_output=True
    )

    return p.returncode == 1
