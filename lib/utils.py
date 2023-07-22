import sys
import os
import json
import getch

from typing import Dict, Optional

from lib.ccommit.data import OPTION_NAME, OPTION_LONG, OPTION_DESC

__DATA_FILE = os.path.realpath(
    f"{os.path.dirname(__file__)}/../data/data.json")

with open(__DATA_FILE, "r") as f:
    SELECTIONS_DATA = json.loads(f.read())

VSCODE_SETTINGS_PATH = ".vscode/settings.json"
VSCODE_SETTINGS = None
if os.path.isfile(VSCODE_SETTINGS_PATH):
    with open(VSCODE_SETTINGS_PATH, "r") as f:
        VSCODE_SETTINGS = json.loads(f.read())


class SelectionException(Exception):
    pass


def build_menu_entry(data_entry: Dict | str) -> str:
    if (isinstance(data_entry, str)):
        return data_entry
    else:
        return f"{data_entry[OPTION_NAME]} ({data_entry[OPTION_LONG]}; {data_entry[OPTION_DESC]})"


def printerr(msg: str):
    print(f"Error: {msg}", file=sys.stderr)


def failure(msg: str, exit_code: int = 1):
    print(f"Failure: {msg}", file=sys.stderr)
    sys.exit(exit_code)


def input_detect_esc(prompt: Optional[str] = None) -> str | None:
    if prompt is not None:
        print(prompt)

    buf = ""
    while True:
        key_stroke = getch.getche()
        if ord(key_stroke) == 27:
            return None
        elif ord(key_stroke) == 10:
            return buf
        else:
            buf += key_stroke
