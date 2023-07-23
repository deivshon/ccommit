from abc import ABC, abstractproperty
from simple_term_menu import TerminalMenu
from lib.utils.data import build_menu_entry
from lib.utils.errors import failure
from lib.utils.input import FinalState, input_detect_esc
from typing import List, Dict, Optional

from lib.utils.data import OPTION_NAME


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

        menu = TerminalMenu(
            entries_flat,
            clear_screen=True,
            title=self.menu_title,
            skip_empty_entries=True,
            search_highlight_style=("fg_black", "bg_green", "bold"),
            menu_highlight_style=("bg_blue", "bold"),
            menu_cursor_style=("fg_blue", "bold")
        )

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
    def __init__(self, prompt: str, len_limit: Optional[int] = None, refuse_empty: bool = False):
        self.prompt = prompt
        self.len_limit = len_limit
        self.refuse_empty = refuse_empty
        self.last_text = ""

    def ask(self):
        input_text = input_detect_esc(
            prompt=self.prompt,
            len_limit=self.len_limit,
            refuse_empty=self.refuse_empty,
            start_text=self.last_text
        )
        self.last_text = input_text.text

        if input_text.state == FinalState.EXITED:
            return None
        else:
            return self.last_text
