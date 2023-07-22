import sys
import os
import json
import curses

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


def input_detect_esc(prompt: Optional[str] = None) -> Optional[str]:
    KEY_ENTER = 10
    KEY_CTRLW = 23
    ESC_KEY = 27
    KEY_BACKSPACE = 127

    default_escdelay = curses.get_escdelay()
    stdscr = curses.initscr()

    curses.noecho()
    curses.set_escdelay(1)
    stdscr.keypad(True)

    if prompt is not None:
        stdscr.addstr(prompt + "\n")

    buf = ""
    while True:
        key = stdscr.getch()

        if key == ESC_KEY:
            buf = None
            break
        elif key == KEY_ENTER:
            break
        elif key == curses.KEY_BACKSPACE or key == KEY_BACKSPACE:
            if len(buf) > 0:
                buf = buf[:-1]
                stdscr.addstr("\b \b")
        elif key == KEY_CTRLW:
            if len(buf) == 0:
                continue
            elif buf[-1] == " ":
                while len(buf) > 0 and buf[-1] == " ":
                    buf = buf[:-1]
                    stdscr.addstr("\b \b")

            while len(buf) > 0 and buf[-1] != " ":
                buf = buf[:-1]
                stdscr.addstr("\b \b")
        else:
            buf += chr(key)
            stdscr.addstr(chr(key))

    stdscr.clear()
    curses.set_escdelay(default_escdelay)
    curses.endwin()

    return buf
