import sys
import subprocess

from lib.builder import ConventionalCommitBuilder


def main():
    cc = ConventionalCommitBuilder()
    commitMessage = cc.interrogate()
    if commitMessage is None:
        sys.exit(0)

    subprocess.run(
        ["git", "commit", "-m", commitMessage.short_message,
            "-m", commitMessage.long_message])
