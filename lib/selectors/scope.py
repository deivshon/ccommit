import json

from typing import List, Optional

from lib.selectors.base import BaseSelector
from lib.utils.input import FinalState, input_detect_esc
from lib.utils.data import VSCODE_SETTINGS, VSCODE_SETTINGS_PATH

NEW_SCOPE = "New scope"
NEW_SCOPE_ONCE = "New scope (only use once)"
NO_SCOPE = "No scope"


class ScopeSelector(BaseSelector):
    __SCOPES_FIELD = "conventionalCommits.scopes"
    __MENU_TITLE = "Select scope"

    def __init__(self):
        self.choice = None
        self.scopeEntries: List[str] = [NO_SCOPE, NEW_SCOPE, NEW_SCOPE_ONCE]
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
    def getNewScope(add=False) -> Optional[str]:
        prompt = "Enter new scope"
        if not add:
            prompt += " (only use once)"

        new_scope = input_detect_esc(
            prompt=prompt, refuse_empty=True)

        if new_scope.state == FinalState.EXITED:
            return None

        if add:
            ScopeSelector.__addScope(new_scope.text)

        return new_scope.text
