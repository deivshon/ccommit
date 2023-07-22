import sys


def failure(msg: str, exit_code: int = 1):
    print(f"Failure: {msg}", file=sys.stderr)
    sys.exit(exit_code)
