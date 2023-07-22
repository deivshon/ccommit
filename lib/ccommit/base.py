from abc import ABC, abstractproperty
from simple_term_menu import TerminalMenu
from lib.utils import SelectionException, build_menu_entry, failure
from typing import List, Dict

from lib.ccommit.data import OPTION_NAME


class BaseSelector(ABC):
    @abstractproperty
    def entries(self) -> List[str] | List[Dict]:
        ...

    @abstractproperty
    def menu_title(self) -> str:
        ...

    def select(self):
        entries_flat = list(
            map(build_menu_entry, self.entries)
        )

        if (len(entries_flat) == 0):
            failure(f"No entries available for {str(self)}")

        menu = TerminalMenu(entries_flat, clear_screen=True,
                            title=self.menu_title)
        selection = menu.show()
        if (isinstance(selection, int)):
            selected_entry = self.entries[selection]
            if isinstance(selected_entry, str):
                return selected_entry
            elif isinstance(selected_entry, dict):
                return selected_entry[OPTION_NAME]
        else:
            raise SelectionException
