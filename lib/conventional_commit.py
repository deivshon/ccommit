from lib.ccommit.type import TypeSelector
from lib.ccommit.scope import ScopeSelector, NEW_SCOPE, NEW_SCOPE_ONCE


class ConventionalCommit:
    def __init__(self):
        self.typeSelector = TypeSelector()
        self.scopeSelector = ScopeSelector()

    def interrogate(self) -> str:
        type = self.typeSelector.select()

        scope = self.scopeSelector.select()
        if scope == NEW_SCOPE:
            scope = ScopeSelector.getNewScope(add=True)
        elif scope == NEW_SCOPE_ONCE:
            scope = ScopeSelector.getNewScope(add=False)

        return f"{type}, {scope}"
