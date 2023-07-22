from abc import ABC, abstractproperty
from simple_term_menu import TerminalMenu
from lib.utils import build_menu_entry, failure, input_detect_esc
from typing import List, Dict, Optional

from lib.ccommit.data import OPTION_NAME


class BaseSelector(ABC):
    @abstractproperty
    def entries(self) -> List[str] | List[Dict]:
        ...

    @abstractproperty
    def menu_title(self) -> str:
        ...

    def select(self) -> Optional[str]:
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
                raise Exception
        else:
            return None


class InputQuestion():
    @staticmethod
    def ask(prompt: Optional[str] = None):
        return input_detect_esc(prompt)
