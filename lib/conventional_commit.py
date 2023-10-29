import os
import sys
import argparse

from lib.utils import git
from lib.config import Config
from lib.utils.errors import failure
from lib.builder import ConventionalCommitBuilder


def main():
    parser = argparse.ArgumentParser(
        prog="ccommit",
        description="Conventional commits from the terminal"
    )

    parser.add_argument(
        "-C", "--working-dir",
        action="store",
        help="Set working directory"
    )

    parser.add_argument(
        "-c", "--config",
        action="store",
        help="Path to the desired configuration file"
    )

    args = parser.parse_args()

    cwd = os.getcwd() if args.working_dir is None else args.working_dir
    if not os.path.isdir(cwd):
        failure(f"Directory `{cwd}` does not exist")
    if not git.inside_repository(cwd):
        failure("Can't commit outside of a git repository")
    if not git.changes_added(cwd):
        failure("Can't commit without changes added")

    if args.config:
        if not os.path.isfile(args.config):
            failure(f"No config at `{os.path.realpath(args.config)}`")

        config = Config.load(args.config)
    else:
        config = Config.load()

    cc = ConventionalCommitBuilder(config)
    commit_message = cc.interrogate()
    if commit_message is None:
        sys.exit(0)

    git.commit(cwd, commit_message)
