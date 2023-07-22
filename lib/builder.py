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
class _SelectorItem():
    selector: BaseSelector | InputQuestion
    postOp: Callable[[str], Optional[str] | Action] | None


@dataclass
class ConventionalCommit():
    short_message: str
    long_message: str


class ConventionalCommitBuilder():
    __TYPE_IDX = 0
    __SCOPE_IDX = 1
    __GITMOJI_IDX = 2
    __SHORT_DESC_IDX = 3
    __LONG_DESC_IDX = 4
    __BREAKING_CHANGES_IDX = 5

    def __init__(self):
        self.typeSelector = TypeSelector()
        self.scopeSelector = ScopeSelector()
        self.gitmojiSelector = GitmojiSelector()
        self.shortDescSelector = InputQuestion(
            "Write a short, imperative tense description of the change", 72, True)
        self.longDescSelector = InputQuestion(
            "Write a longer description of the change", None, False)
        self.breakingChangesSelector = InputQuestion(
            "List any breaking changes or issues closed by this change", None, False)

        self.selectors: List[_SelectorItem] = [
            _SelectorItem(self.typeSelector, None),
            _SelectorItem(self.scopeSelector,
                          ConventionalCommitBuilder.__post_scope),
            _SelectorItem(self.gitmojiSelector,
                          ConventionalCommitBuilder.__post_gitmoji),
            _SelectorItem(self.shortDescSelector, None),
            _SelectorItem(self.longDescSelector, None),
            _SelectorItem(self.breakingChangesSelector, None)
        ]

        self.type = None
        self.scope = None

    def interrogate(self) -> Optional[ConventionalCommit]:
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

        return ConventionalCommit(
            short_message=ConventionalCommitBuilder.__build_short_commit_message(
                results[ConventionalCommitBuilder.__TYPE_IDX],
                results[ConventionalCommitBuilder.__SCOPE_IDX],
                results[ConventionalCommitBuilder.__GITMOJI_IDX],
                results[ConventionalCommitBuilder.__SHORT_DESC_IDX]
            ),

            long_message=ConventionalCommitBuilder.__build_long_commit_message(
                results[ConventionalCommitBuilder.__LONG_DESC_IDX],
                results[ConventionalCommitBuilder.__BREAKING_CHANGES_IDX],
            )
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
    def __build_short_commit_message(type: str, scope: str, gitmoji: str, short_description: str):
        commit_message = f"{type}"

        if len(scope) != 0:
            commit_message += f"({scope})"
        commit_message += ":"

        if len(gitmoji) != 0:
            commit_message += f" {gitmoji}"

        commit_message += f" {short_description}"
        return commit_message

    @staticmethod
    def __build_long_commit_message(long_description: str, breaking_changes: str):
        if len(long_description) == 0 or len(breaking_changes) == 0:
            return f"{long_description}{breaking_changes}"
        else:
            return f"{long_description}\n{breaking_changes}"
