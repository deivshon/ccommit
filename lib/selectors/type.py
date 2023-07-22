from lib.utils.data import SELECTIONS_DATA
from lib.selectors.base import BaseSelector


class TypeSelector(BaseSelector):
    __MENU_TITLE = "Select commit type"

    def __init__(self):
        self.choice = None

    @property
    def entries(self):
        return SELECTIONS_DATA["type"]

    @property
    def menu_title(self) -> str:
        return self.__MENU_TITLE
