from lib.builder import ConventionalCommitBuilder


def main():
    cc = ConventionalCommitBuilder()
    commitMessage = cc.interrogate()
    print(commitMessage)
