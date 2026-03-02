"""Cupcake — Policy enforcement for AI agents and automation tools.

Python bindings for the Cupcake policy engine, enabling custom applications
to embed OPA/Rego policy evaluation for governance and safety.

Usage::

    from cupcake import Cupcake

    cupcake = Cupcake()
    await cupcake.init(".cupcake")

    decision = await cupcake.evaluate({
        "kind": "shell",
        "command": "rm -rf /",
        "user": "agent",
    })

    if decision["decision"] == "Deny":
        raise RuntimeError(f"Blocked: {decision['reason']}")

Both class-based (multiple policy sets) and module-level (singleton) APIs
are provided, mirroring the TypeScript bindings.
"""

from __future__ import annotations

import asyncio
import json
import logging
from concurrent.futures import ThreadPoolExecutor
from enum import Enum
from typing import Any, Literal

logger = logging.getLogger("cupcake")

__all__ = [
    "Cupcake",
    "CupcakeError",
    "Decision",
    "HookEvent",
    "Severity",
    "init",
    "init_sync",
    "evaluate",
    "evaluate_sync",
    "version",
    "is_ready",
]

# Type aliases
HookEvent = dict[str, Any]
"""Hook event input — a generic dict your application defines.

Cupcake doesn't enforce a specific event schema for embedded use.
Your policies evaluate whatever structure you provide.

Example::

    event: HookEvent = {
        "kind": "tool_use",
        "tool": "database_query",
        "query": "DELETE FROM users",
        "user_id": "agent-123",
    }
"""


class Severity(str, Enum):
    """Severity levels for policy decisions."""

    LOW = "LOW"
    MEDIUM = "MEDIUM"
    HIGH = "HIGH"
    CRITICAL = "CRITICAL"


# Decision is a TypedDict-like structure returned as a plain dict.
# We keep it as dict[str, Any] for flexibility (policies can add fields).
Decision = dict[str, Any]
"""Policy decision returned from evaluation.

Standard fields::

    {
        "decision": "Allow" | "Deny" | "Halt" | "Ask" | "Modify",
        "reason": "...",           # Human-readable reason
        "context": ["..."],        # Guidance (for Allow)
        "severity": "HIGH",        # LOW / MEDIUM / HIGH / CRITICAL
        "rule_id": "SEC-001",      # Rule that triggered
        "updated_input": {...},    # Modified input (for Modify)
    }
"""

# Shared thread pool for async operations (mirrors libuv threadpool in TS)
_executor = ThreadPoolExecutor(max_workers=4, thread_name_prefix="cupcake")


class CupcakeError(Exception):
    """Error raised by the Cupcake engine.

    Attributes:
        code: Machine-readable error code.
        cause: Original exception, if any.
    """

    def __init__(
        self,
        message: str,
        code: str,
        cause: BaseException | None = None,
    ) -> None:
        super().__init__(message)
        self.code = code
        self.cause = cause
        if cause is not None:
            self.__cause__ = cause


class Cupcake:
    """Main Cupcake class for policy evaluation.

    Wraps the native Rust engine and provides a Python-friendly API.
    Create multiple instances to evaluate against different policy sets.

    Example::

        # Instance-based API (recommended for multiple policy sets)
        production = Cupcake()
        await production.init("./policies/production")

        staging = Cupcake()
        await staging.init("./policies/staging")

        await production.evaluate(event)
        await staging.evaluate(event)
    """

    def __init__(self) -> None:
        self._engine: Any | None = None
        self._initialized: bool = False

    async def init(
        self,
        path: str = ".cupcake",
        harness: Literal["claude", "cursor", "factory", "opencode"] = "claude",
    ) -> None:
        """Initialize the Cupcake engine.

        This method:
        1. Ensures the OPA binary is installed (auto-downloads if needed).
        2. Loads and compiles policies from the specified directory.
        3. Initializes the WASM runtime.

        The heavy lifting runs in a thread pool to avoid blocking the
        event loop.

        Args:
            path: Path to project directory or ``.cupcake`` folder.
            harness: Harness type for policy namespace.

        Raises:
            CupcakeError: If already initialized or initialization fails.
        """
        if self._initialized:
            raise CupcakeError("Cupcake already initialized", "ALREADY_INITIALIZED")

        try:
            from ._installer import ensure_opa_installed

            await ensure_opa_installed()

            loop = asyncio.get_running_loop()
            engine = await loop.run_in_executor(
                _executor, _create_engine, path, harness
            )
            self._engine = engine
            self._initialized = True
        except CupcakeError:
            raise
        except Exception as exc:
            raise CupcakeError(
                f"Failed to initialize Cupcake: {exc}", "INIT_FAILED", exc
            ) from exc

    def init_sync(
        self,
        path: str = ".cupcake",
        harness: Literal["claude", "cursor", "factory", "opencode"] = "claude",
    ) -> None:
        """Initialize the Cupcake engine synchronously (blocks the thread).

        Warning:
            This method blocks the calling thread during initialization.
            Only use in CLI scripts or top-level startup code before an
            event loop starts.  For async applications, use :meth:`init`.

        Args:
            path: Path to project directory or ``.cupcake`` folder.
            harness: Harness type for policy namespace.

        Raises:
            CupcakeError: If already initialized or initialization fails.
        """
        if self._initialized:
            raise CupcakeError("Cupcake already initialized", "ALREADY_INITIALIZED")

        try:
            from ._installer import ensure_opa_installed_sync

            ensure_opa_installed_sync()

            self._engine = _create_engine(path, harness)
            self._initialized = True
        except CupcakeError:
            raise
        except Exception as exc:
            raise CupcakeError(
                f"Failed to initialize Cupcake: {exc}", "INIT_FAILED", exc
            ) from exc

    async def evaluate(self, event: HookEvent) -> Decision:
        """Evaluate a hook event asynchronously (recommended, non-blocking).

        Runs policy evaluation in a thread pool so the event loop stays
        responsive.

        Args:
            event: Hook event dict (your application defines the structure).

        Returns:
            Policy decision dict.

        Raises:
            CupcakeError: If not initialized or evaluation fails.
        """
        if self._engine is None:
            raise CupcakeError(
                "Cupcake not initialized. Call init() first.", "NOT_INITIALIZED"
            )

        try:
            input_json = json.dumps(event)
            loop = asyncio.get_running_loop()
            result_json = await loop.run_in_executor(
                _executor, self._engine.evaluate, input_json
            )
            return json.loads(result_json)
        except CupcakeError:
            raise
        except Exception as exc:
            raise CupcakeError(
                f"Policy evaluation failed: {exc}", "EVALUATION_FAILED", exc
            ) from exc

    def evaluate_sync(self, event: HookEvent) -> Decision:
        """Evaluate a hook event synchronously (blocks the thread).

        Warning:
            This method blocks the calling thread until evaluation
            completes.  For async applications, use :meth:`evaluate`.

        Args:
            event: Hook event dict.

        Returns:
            Policy decision dict.

        Raises:
            CupcakeError: If not initialized or evaluation fails.
        """
        if self._engine is None:
            raise CupcakeError(
                "Cupcake not initialized. Call init_sync() first.",
                "NOT_INITIALIZED",
            )

        try:
            input_json = json.dumps(event)
            result_json = self._engine.evaluate(input_json)
            return json.loads(result_json)
        except CupcakeError:
            raise
        except Exception as exc:
            raise CupcakeError(
                f"Policy evaluation failed: {exc}", "EVALUATION_FAILED", exc
            ) from exc

    @property
    def version(self) -> str:
        """The Cupcake engine version string."""
        if self._engine is None:
            raise CupcakeError("Cupcake not initialized", "NOT_INITIALIZED")
        return self._engine.version()

    @property
    def is_ready(self) -> bool:
        """Whether the engine is ready to evaluate policies."""
        if self._engine is None:
            return False
        return self._engine.is_ready()

    def __repr__(self) -> str:
        if self._initialized:
            return f"Cupcake(version={self.version!r}, ready={self.is_ready})"
        return "Cupcake(initialized=False)"


# ---------------------------------------------------------------------------
# Module-level singleton API (convenience, mirrors TS module functions)
# ---------------------------------------------------------------------------

_default_instance: Cupcake | None = None


async def init(
    path: str = ".cupcake",
    harness: Literal["claude", "cursor", "factory", "opencode"] = "claude",
) -> None:
    """Initialize the default Cupcake instance.

    Convenience function for the module-level API.  For multiple policy
    sets, use the class-based API instead.

    Example::

        import cupcake
        await cupcake.init(".cupcake")
        decision = await cupcake.evaluate(event)
    """
    global _default_instance  # noqa: PLW0603
    _default_instance = Cupcake()
    await _default_instance.init(path, harness)


def init_sync(
    path: str = ".cupcake",
    harness: Literal["claude", "cursor", "factory", "opencode"] = "claude",
) -> None:
    """Initialize the default Cupcake instance synchronously.

    Warning:
        Blocks the calling thread.  Use :func:`init` for async code.
    """
    global _default_instance  # noqa: PLW0603
    _default_instance = Cupcake()
    _default_instance.init_sync(path, harness)


async def evaluate(event: HookEvent) -> Decision:
    """Evaluate an event using the default instance (async, non-blocking)."""
    if _default_instance is None:
        raise CupcakeError(
            "Cupcake not initialized. Call init() first.", "NOT_INITIALIZED"
        )
    return await _default_instance.evaluate(event)


def evaluate_sync(event: HookEvent) -> Decision:
    """Evaluate an event using the default instance (sync, blocks thread)."""
    if _default_instance is None:
        raise CupcakeError(
            "Cupcake not initialized. Call init_sync() first.",
            "NOT_INITIALIZED",
        )
    return _default_instance.evaluate_sync(event)


def version() -> str:
    """Get the Cupcake engine version from the default instance."""
    if _default_instance is None:
        raise CupcakeError("Cupcake not initialized", "NOT_INITIALIZED")
    return _default_instance.version


def is_ready() -> bool:
    """Check if the default instance is ready."""
    if _default_instance is None:
        return False
    return _default_instance.is_ready


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


def _create_engine(path: str, harness: str) -> Any:
    """Create a native PolicyEngine (called from thread pool)."""
    from ._native import PolicyEngine

    return PolicyEngine(path, harness)
