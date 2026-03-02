"""Basic tests for Cupcake Python bindings.

Mirrors ``cupcake-ts/__test__/basic.test.ts``.

Tests are structured in two tiers:

- **Engine tests** require OPA with WASM compilation support.  OPA's static
  ARM64 builds lack WASM, so these are skipped on platforms where policy
  compilation is unavailable.  They always pass on x86_64 CI.
- **Pure-Python tests** (error classes, repr, not-initialized guards) run
  everywhere the native module is installed.
"""

from __future__ import annotations

import asyncio
import os
import shutil
import tempfile
from pathlib import Path

import pytest

from cupcake import Cupcake, CupcakeError

# ---------------------------------------------------------------------------
# External test-fixtures (shared with cupcake-ts, may not exist)
# ---------------------------------------------------------------------------

TEST_CUPCAKE_DIR = str(
    Path(__file__).resolve().parent.parent.parent
    / "test-fixtures"
    / ".cupcake"
)

_HAS_FIXTURES = os.path.isdir(TEST_CUPCAKE_DIR)
needs_fixtures = pytest.mark.skipif(
    not _HAS_FIXTURES, reason=f"test fixtures not found at {TEST_CUPCAKE_DIR}"
)

# ---------------------------------------------------------------------------
# Skip everything if the native module isn't built
# ---------------------------------------------------------------------------

try:
    from cupcake._native import PolicyEngine as _PE  # noqa: F401
    _HAS_NATIVE = True
except ImportError:
    _HAS_NATIVE = False

pytestmark = pytest.mark.skipif(
    not _HAS_NATIVE, reason="cupcake._native not built (run `maturin develop` first)"
)


# ---------------------------------------------------------------------------
# Minimal inline fixture — mirrors cupcake-core/tests/common/mod.rs
# The system evaluate.rego uses walk() which requires OPA WASM support.
# ---------------------------------------------------------------------------

_MINIMAL_POLICY_REGO = """\
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.policies.minimal

import rego.v1

deny contains decision if {
    input.test_condition == "never_match_12345"
    decision := {
        "reason": "Test policy that never triggers",
        "severity": "LOW",
        "rule_id": "TEST-001"
    }
}
"""

_SYSTEM_EVALUATE_REGO = """\
package cupcake.system

import rego.v1

# METADATA
# scope: document
# title: System Aggregation Entrypoint
# custom:
#   entrypoint: true
#   routing:
#     required_events: []
#     required_tools: []

evaluate := decision_set if {
    decision_set := {
        "halts": collect_verbs("halt"),
        "denials": collect_verbs("deny"),
        "blocks": collect_verbs("block"),
        "asks": collect_verbs("ask"),
        "modifications": collect_verbs("modify"),
        "add_context": collect_verbs("add_context")
    }
}

collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]
    result := all_decisions
}

default collect_verbs(_) := []
"""


@pytest.fixture(scope="session")
def tmp_cupcake_dir() -> str:
    """Create a minimal .cupcake project (same layout as Rust integration tests)."""
    with tempfile.TemporaryDirectory(prefix="cupcake-pytest-") as tmp:
        root = Path(tmp)
        cupcake = root / ".cupcake"
        harness_dir = cupcake / "policies" / "claude"
        system_dir = harness_dir / "system"
        signals_dir = cupcake / "signals"
        system_dir.mkdir(parents=True)
        signals_dir.mkdir(parents=True)
        (cupcake / "rulebook.yml").write_text("signals: {}\nbuiltins: {}")
        (harness_dir / "minimal.rego").write_text(_MINIMAL_POLICY_REGO)
        (system_dir / "evaluate.rego").write_text(_SYSTEM_EVALUATE_REGO)
        yield str(root)


def _can_init_engine(path: str) -> bool:
    """Try to create an engine — returns False if OPA WASM compilation unavailable."""
    try:
        from cupcake._native import PolicyEngine
        PolicyEngine(path, "claude")
        return True
    except RuntimeError:
        return False


@pytest.fixture(scope="session")
def engine_available(tmp_cupcake_dir: str) -> bool:
    return _can_init_engine(tmp_cupcake_dir)


def _skip_unless_engine(engine_available: bool) -> None:
    if not engine_available:
        pytest.skip(
            "OPA WASM compilation unavailable on this platform "
            "(ARM64 static builds lack WASM support)"
        )


# ---- Initialization ---------------------------------------------------------


class TestInitialization:
    async def test_init_async(self, tmp_cupcake_dir: str, engine_available: bool) -> None:
        _skip_unless_engine(engine_available)
        cupcake = Cupcake()
        await cupcake.init(tmp_cupcake_dir)
        assert cupcake.is_ready is True
        assert cupcake.version

    def test_init_sync(self, tmp_cupcake_dir: str, engine_available: bool) -> None:
        _skip_unless_engine(engine_available)
        cupcake = Cupcake()
        cupcake.init_sync(tmp_cupcake_dir)
        assert cupcake.is_ready is True

    async def test_double_init_raises(self, tmp_cupcake_dir: str, engine_available: bool) -> None:
        _skip_unless_engine(engine_available)
        cupcake = Cupcake()
        await cupcake.init(tmp_cupcake_dir)
        with pytest.raises(CupcakeError, match="already initialized"):
            await cupcake.init(tmp_cupcake_dir)

    async def test_invalid_path_raises(self) -> None:
        cupcake = Cupcake()
        with pytest.raises(CupcakeError):
            await cupcake.init("/nonexistent/path")

    def test_not_initialized_repr(self) -> None:
        cupcake = Cupcake()
        assert "initialized=False" in repr(cupcake)

    def test_initialized_repr(self, tmp_cupcake_dir: str, engine_available: bool) -> None:
        _skip_unless_engine(engine_available)
        cupcake = Cupcake()
        cupcake.init_sync(tmp_cupcake_dir)
        r = repr(cupcake)
        assert "version=" in r
        assert "ready=True" in r


# ---- Evaluation --------------------------------------------------------------


class TestEvaluation:
    @pytest.fixture(autouse=True)
    async def _init_cupcake(self, tmp_cupcake_dir: str, engine_available: bool) -> None:
        _skip_unless_engine(engine_available)
        self.cupcake = Cupcake()
        await self.cupcake.init(tmp_cupcake_dir)

    async def test_evaluate_async(self) -> None:
        event = {
            "hookEventName": "PreToolUse",
            "tool_name": "Bash",
            "command": "ls",
            "args": ["-la"],
        }
        decision = await self.cupcake.evaluate(event)
        assert isinstance(decision, dict)
        assert any(
            k in decision for k in ("Allow", "Deny", "Halt", "Ask", "Modify")
        )

    def test_evaluate_sync(self) -> None:
        event = {
            "hookEventName": "PreToolUse",
            "tool_name": "Bash",
            "command": "ls",
            "args": ["-la"],
        }
        decision = self.cupcake.evaluate_sync(event)
        assert isinstance(decision, dict)
        assert any(
            k in decision for k in ("Allow", "Deny", "Halt", "Ask", "Modify")
        )

    async def test_custom_event(self) -> None:
        event = {
            "hookEventName": "UserPromptSubmit",
            "prompt": "test prompt",
            "user": "test-user",
        }
        decision = await self.cupcake.evaluate(event)
        assert isinstance(decision, dict)

    async def test_concurrent_evaluations(self) -> None:
        events = [
            {
                "hookEventName": "PreToolUse",
                "tool_name": "Bash",
                "command": "echo",
                "args": [f"test-{i}"],
            }
            for i in range(10)
        ]
        decisions = await asyncio.gather(
            *(self.cupcake.evaluate(e) for e in events)
        )
        assert len(decisions) == 10
        for d in decisions:
            assert isinstance(d, dict)


class TestEvaluationNoInit:
    async def test_evaluate_before_init_raises(self) -> None:
        uninit = Cupcake()
        with pytest.raises(CupcakeError, match="not initialized"):
            await uninit.evaluate({"hookEventName": "PreToolUse"})


# ---- Module-level API --------------------------------------------------------


class TestModuleAPI:
    async def test_module_level_functions(self, tmp_cupcake_dir: str, engine_available: bool) -> None:
        _skip_unless_engine(engine_available)
        import cupcake

        await cupcake.init(tmp_cupcake_dir)
        assert cupcake.is_ready() is True
        assert cupcake.version()

        decision = await cupcake.evaluate({
            "hookEventName": "PreToolUse",
            "tool_name": "Bash",
            "command": "test",
        })
        assert isinstance(decision, dict)


# ---- Evaluation with shared test-fixtures (optional) -------------------------


@needs_fixtures
class TestExternalFixtures:
    @pytest.fixture(autouse=True)
    async def _init_cupcake(self, engine_available: bool) -> None:
        _skip_unless_engine(engine_available)
        self.cupcake = Cupcake()
        await self.cupcake.init(TEST_CUPCAKE_DIR)

    async def test_evaluate_external(self) -> None:
        decision = await self.cupcake.evaluate({
            "hookEventName": "PreToolUse",
            "tool_name": "Bash",
            "command": "ls -la",
        })
        assert isinstance(decision, dict)


# ---- Error handling ----------------------------------------------------------


class TestErrors:
    async def test_error_has_code(self) -> None:
        cupcake = Cupcake()
        try:
            await cupcake.init("/invalid/path")
            pytest.fail("Should have raised CupcakeError")
        except CupcakeError as exc:
            assert exc.code
            assert str(exc)

    def test_cupcake_error_attributes(self) -> None:
        err = CupcakeError("test message", "TEST_CODE")
        assert err.code == "TEST_CODE"
        assert err.cause is None
        assert str(err) == "test message"

    def test_cupcake_error_with_cause(self) -> None:
        cause = ValueError("original")
        err = CupcakeError("wrapped", "WRAP", cause)
        assert err.cause is cause
        assert err.__cause__ is cause


# ---- Version -----------------------------------------------------------------


class TestVersion:
    async def test_version_string(self, tmp_cupcake_dir: str, engine_available: bool) -> None:
        _skip_unless_engine(engine_available)
        cupcake = Cupcake()
        await cupcake.init(tmp_cupcake_dir)
        v = cupcake.version
        assert isinstance(v, str)
        assert len(v) > 0
        assert "cupcake" in v.lower()

    def test_version_before_init_raises(self) -> None:
        cupcake = Cupcake()
        with pytest.raises(CupcakeError, match="not initialized"):
            _ = cupcake.version
