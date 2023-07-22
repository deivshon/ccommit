from typing import Callable, List, Optional
from lib.ccommit.base import BaseSelector, InputQuestion
from lib.ccommit.type import TypeSelector
from lib.ccommit.scope import ScopeSelector, NO_SCOPE, NEW_SCOPE, NEW_SCOPE_ONCE
from enum import Enum


class Action(Enum):
    REPEAT = 0


class ConventionalCommit:
    __TYPE_IDX = 0
    __SCOPE_IDX = 1

    def __init__(self):
        self.typeSelector = TypeSelector()
        self.scopeSelector = ScopeSelector()

        self.selectors: List[BaseSelector | InputQuestion] = [
            self.typeSelector,
            self.scopeSelector
        ]
        self.selectorsOps: List[Callable[[str], Optional[str] | Action] | None] = [
            None,
            ConventionalCommit.__post_scope
        ]

        self.type = None
        self.scope = None

    def interrogate(self) -> Optional[str]:
        i = 0
        results = [""] * len(self.selectors)

        while i < len(self.selectors):
            currentSelector = self.selectors[i]
            if isinstance(currentSelector, BaseSelector):
                currentResult = currentSelector.select()
            else:
                currentResult = currentSelector.ask()

            postOpFunction = self.selectorsOps[i]
            if currentResult is not None and postOpFunction is not None:
                currentResult = postOpFunction(currentResult)

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
            results[ConventionalCommit.__SCOPE_IDX]
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
    def __build_conventional_commit(type: str, scope: str):
        commit_message = f"{type}"

        if len(scope) != 0:
            commit_message += f"({scope})"

        return commit_message
