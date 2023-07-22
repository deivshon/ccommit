import json
from lib.utils import VSCODE_SETTINGS, VSCODE_SETTINGS_PATH
from typing import List
from lib.ccommit.base import BaseSelector

NEW_SCOPE = "New scope"
NEW_SCOPE_ONCE = "New scope (only use once)"


class ScopeSelector(BaseSelector):
    __SCOPES_FIELD = "conventionalCommits.scopes"
    __MENU_TITLE = "Select scope"

    def __init__(self):
        self.choice = None
        self.scopeEntries: List[str] = [NEW_SCOPE, NEW_SCOPE_ONCE]
        self.__init_entries()

    def __init_entries(self):
        if VSCODE_SETTINGS is None:
            return
        if self.__SCOPES_FIELD not in VSCODE_SETTINGS:
            return

        self.scopeEntries = VSCODE_SETTINGS[self.__SCOPES_FIELD] + \
            self.scopeEntries

    @property
    def entries(self):
        return self.scopeEntries

    @property
    def menu_title(self) -> str:
        return self.__MENU_TITLE

    @staticmethod
    def __addScope(scope: str):
        with open(VSCODE_SETTINGS_PATH, "r") as f:
            currentSettings = json.loads(f.read())

        if (ScopeSelector.__SCOPES_FIELD not in currentSettings):
            currentSettings[ScopeSelector.__SCOPES_FIELD] = []

        currentSettings[ScopeSelector.__SCOPES_FIELD] += [scope]
        with open(VSCODE_SETTINGS_PATH, "w") as f:
            f.write(json.dumps(currentSettings, indent=4))

    @staticmethod
    def getNewScope(add=False) -> str:
        newScope = input()

        if add:
            ScopeSelector.__addScope(newScope)

        return newScope
