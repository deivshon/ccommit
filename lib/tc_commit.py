from lib.conventional_commit import ConventionalCommit


def main():
    cc = ConventionalCommit()
    commitMessage = cc.interrogate()
    print(commitMessage)
