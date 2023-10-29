from logging import warning
import os
import json

from typing import Optional, Self
from dataclasses import dataclass

from lib.utils.errors import failure


@dataclass
class Config():
    __DEFAULT_PATH = os.path.join(os.path.expanduser(
        "~"), ".config", "ccommit", "config.json")

    use_emojis: bool = False

    @classmethod
    def load(cls, path: str = __DEFAULT_PATH) -> Self:
        if not os.path.isfile(path):
            return cls()

        with open(path) as f:
            try:
                return cls(**json.loads(f.read()))
            except Exception as e:
                failure(f"Could not parse configuration file at `{path}`: {e}")
