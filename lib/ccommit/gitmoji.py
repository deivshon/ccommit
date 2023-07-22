from lib.utils import SELECTIONS_DATA
from lib.ccommit.base import BaseSelector

NO_GITMOJI = "None"


class GitmojiSelector(BaseSelector):
    __MENU_TITLE = "Select gitmoji"

    def __init__(self):
        self.choice = None

    @property
    def entries(self):
        return SELECTIONS_DATA["gitmoji"]

    @property
    def menu_title(self) -> str:
        return self.__MENU_TITLE
