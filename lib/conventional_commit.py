import os
import sys

from lib.utils import git
from lib.utils.errors import failure
from lib.builder import ConventionalCommitBuilder


def main():
    cwd = os.getcwd()
    if not git.inside_repository(cwd):
        failure("Can't commit outside of a git repository")
    if not git.changes_added(cwd):
        failure("Can't commit without changes added")

    cc = ConventionalCommitBuilder()
    commit_message = cc.interrogate()
    if commit_message is None:
        sys.exit(0)

    git.commit(cwd, commit_message)
