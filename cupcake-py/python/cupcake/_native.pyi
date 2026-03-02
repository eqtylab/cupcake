"""Type stubs for the native Rust extension module.

These stubs provide type information for ``cupcake._native``, the
PyO3-compiled extension.  They are used by type checkers (mypy, pyright)
and IDEs for autocompletion.
"""

class PolicyEngine:
    """Native policy engine wrapping the Cupcake Rust core.

    Args:
        path: Path to project directory or ``.cupcake`` folder.
        harness: Harness type (``"claude"``, ``"cursor"``, ``"factory"``,
                 ``"opencode"``).  Defaults to ``"claude"``.

    Raises:
        RuntimeError: If initialization fails.
    """

    def __init__(self, path: str, harness: str | None = None) -> None: ...

    def evaluate(self, input_json: str) -> str:
        """Evaluate a hook event.

        Args:
            input_json: JSON string containing the hook event.

        Returns:
            JSON string with the policy decision.

        Raises:
            ValueError: If ``input_json`` is not valid JSON.
            RuntimeError: If evaluation fails.
        """
        ...

    def version(self) -> str:
        """Return the Cupcake engine version string."""
        ...

    def is_ready(self) -> bool:
        """Return whether the engine is ready to evaluate policies."""
        ...

    def __repr__(self) -> str: ...
