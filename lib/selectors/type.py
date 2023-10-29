from typing import Dict
from lib.selectors.base import BaseSelector
from lib.utils.data import OPTION_NAME, SELECTIONS_DATA


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

    def selection_from_dict(self, entry: Dict) -> str:
        return entry[OPTION_NAME]
