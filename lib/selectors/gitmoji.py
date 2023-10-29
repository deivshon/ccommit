from typing import Dict
from lib.selectors.base import BaseSelector
from lib.utils.data import SELECTIONS_DATA, OPTION_NAME, OPTION_EMOJI

NO_GITMOJI = "None"


class GitmojiSelector(BaseSelector):
    __MENU_TITLE = "Select gitmoji"

    def __init__(self, use_emojis: bool):
        self.choice = None
        self.use_emojis = use_emojis

    @property
    def entries(self):
        return SELECTIONS_DATA["gitmoji"]

    @property
    def menu_title(self) -> str:
        return self.__MENU_TITLE

    def selection_from_dict(self, entry: Dict) -> str:
        return entry[OPTION_EMOJI] if self.use_emojis else entry[OPTION_NAME]
