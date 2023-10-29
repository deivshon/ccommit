import sys
from typing import NoReturn


def failure(msg: str, exit_code: int = 1) -> NoReturn:
    print(f"Failure: {msg}", file=sys.stderr)
    sys.exit(exit_code)
