from dataclasses import dataclass
from typing import Callable, List, Optional
from lib.ccommit.base import BaseSelector, InputQuestion
from lib.ccommit.type import TypeSelector
from lib.ccommit.scope import ScopeSelector, NO_SCOPE, NEW_SCOPE, NEW_SCOPE_ONCE
from lib.ccommit.gitmoji import NO_GITMOJI, GitmojiSelector
from enum import Enum


class Action(Enum):
    REPEAT = 0


@dataclass
class SelectorItem():
    selector: BaseSelector | InputQuestion
    postOp: Callable[[str], Optional[str] | Action] | None


class ConventionalCommit:
    __TYPE_IDX = 0
    __SCOPE_IDX = 1
    __GITMOJI_IDX = 2

    def __init__(self):
        self.typeSelector = TypeSelector()
        self.scopeSelector = ScopeSelector()
        self.gitmojiSelector = GitmojiSelector()

        self.selectors: List[SelectorItem] = [
            SelectorItem(self.typeSelector, None),
            SelectorItem(self.scopeSelector, ConventionalCommit.__post_scope),
            SelectorItem(self.gitmojiSelector,
                         ConventionalCommit.__post_gitmoji)
        ]

        self.type = None
        self.scope = None

    def interrogate(self) -> Optional[str]:
        i = 0
        results = [""] * len(self.selectors)

        while i < len(self.selectors):
            currentSelector = self.selectors[i].selector
            currentPostOp = self.selectors[i].postOp

            if isinstance(currentSelector, BaseSelector):
                currentResult = currentSelector.select()
            else:
                currentResult = currentSelector.ask()

            if currentResult is not None and currentPostOp is not None:
                currentResult = currentPostOp(currentResult)

            if currentResult == Action.REPEAT:
                continue

            if currentResult is None:
                if i == 0:
                    return None
                else:
                    i -= 1
                    continue
            else:
                results[i] = currentResult
                i += 1

        return ConventionalCommit.__build_conventional_commit(
            results[ConventionalCommit.__TYPE_IDX],
            results[ConventionalCommit.__SCOPE_IDX],
            results[ConventionalCommit.__GITMOJI_IDX]
        )

    @staticmethod
    def __post_scope(selected: str) -> Optional[str] | Action:
        scope = selected

        if selected == NO_SCOPE:
            scope = ""
        if selected == NEW_SCOPE:
            scope = ScopeSelector.getNewScope(add=True)
        elif selected == NEW_SCOPE_ONCE:
            scope = ScopeSelector.getNewScope(add=False)

        if scope is None:
            return Action.REPEAT
        else:
            return scope

    @staticmethod
    def __post_gitmoji(selected: str):
        gitmoji = selected

        if selected == NO_GITMOJI:
            gitmoji = ""

        return gitmoji

    @staticmethod
    def __build_conventional_commit(type: str, scope: str, gitmoji: Optional[str]):
        commit_message = f"{type}"

        if len(scope) != 0:
            commit_message += f"({scope})"
        commit_message += ":"

        if gitmoji is not None:
            commit_message += f" {gitmoji}"

        return commit_message
