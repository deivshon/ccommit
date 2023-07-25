import os
import json

from typing import Dict

OPTION_NAME = "name"
OPTION_LONG = "long"
OPTION_DESC = "description"

COMMIT_TYPE = "type"

__DATA_FILE = os.path.realpath(
    f"{os.path.dirname(__file__)}/../../data/data.json")

with open(__DATA_FILE, "r") as f:
    SELECTIONS_DATA = json.loads(f.read())

VSCODE_SETTINGS_PATH = ".vscode/settings.json"
VSCODE_SETTINGS = None
if os.path.isfile(VSCODE_SETTINGS_PATH):
    with open(VSCODE_SETTINGS_PATH, "r") as f:
        VSCODE_SETTINGS = json.loads(f.read())


def build_menu_entry(data_entry: Dict | str) -> str:
    if (isinstance(data_entry, str)):
        return data_entry
    else:
        entry = data_entry[OPTION_NAME]
        if len(data_entry[OPTION_LONG]) == 0:
            entry += f" ({data_entry[OPTION_DESC]})"
        elif len(data_entry[OPTION_DESC]) == 0:
            entry += f" ({data_entry[OPTION_LONG]})"
        else:
            entry += f" ({data_entry[OPTION_LONG]}; {data_entry[OPTION_DESC]})"

        return entry
