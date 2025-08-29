"""
Cupcake - Python bindings for the Cupcake policy engine

This package provides a Pythonic interface to the Cupcake policy engine for
governance and augmentation of agentic AI systems.

Basic Usage:
    import cupcake
    
    # Initialize with project directory
    cupcake.init(".cupcake")
    
    # Evaluate a hook event
    result = cupcake.eval({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "command": "rm -rf /"
    })
    
    if result["decision"] == "block":
        print(f"Blocked: {result['reason']}")

Async Usage:
    import asyncio
    import cupcake
    
    async def main():
        await cupcake.init_async(".cupcake")
        result = await cupcake.eval_async({
            "hookEventName": "PreToolUse",
            "tool_name": "Bash"
        })
"""

import asyncio
import json
import os
from typing import Dict, Any, Optional
from pathlib import Path

# Import the native Rust module
try:
    from .cupcake_native import PolicyEngine
except ImportError as e:
    raise ImportError(
        "Failed to import native Cupcake module. "
        "Please ensure the package was built with maturin. "
        f"Original error: {e}"
    )

# Import the OPA installer
from .installer import ensure_opa_installed, OPA_VERSION


class Cupcake:
    """
    Main Cupcake engine class
    
    This class provides both sync and async interfaces to the policy engine.
    It manages the lifecycle of the underlying Rust engine and handles
    OPA binary installation.
    """
    
    def __init__(self):
        """Initialize an unconnected Cupcake instance"""
        self._engine: Optional[PolicyEngine] = None
        self._project_path: Optional[str] = None
    
    def init(self, path: str = ".cupcake") -> None:
        """
        Initialize the policy engine synchronously
        
        Args:
            path: Path to the project directory or .cupcake folder
        
        Raises:
            RuntimeError: If initialization fails
        """
        # Ensure OPA is installed
        ensure_opa_installed()
        
        # Resolve the path
        resolved_path = self._resolve_path(path)
        
        # Initialize the engine
        self._engine = PolicyEngine(resolved_path)
        self._project_path = resolved_path
    
    async def init_async(self, path: str = ".cupcake") -> None:
        """
        Initialize the policy engine asynchronously
        
        Args:
            path: Path to the project directory or .cupcake folder
        
        Raises:
            RuntimeError: If initialization fails
        """
        # Run OPA installation in thread pool
        await asyncio.to_thread(ensure_opa_installed)
        
        # Resolve the path
        resolved_path = self._resolve_path(path)
        
        # Initialize the engine in thread pool
        self._engine = await asyncio.to_thread(PolicyEngine, resolved_path)
        self._project_path = resolved_path
    
    def eval(self, event: Dict[str, Any]) -> Dict[str, Any]:
        """
        Evaluate a hook event synchronously
        
        Args:
            event: The hook event dictionary
        
        Returns:
            Decision dictionary with 'decision' and optional 'reason'
        
        Raises:
            RuntimeError: If engine not initialized
            ValueError: If event is invalid
        """
        if not self._engine:
            raise RuntimeError(
                "Cupcake engine not initialized. "
                "Call init() or init_async() first."
            )
        
        # Convert event to JSON
        event_json = json.dumps(event)
        
        # Evaluate (GIL is released in Rust)
        result_json = self._engine.evaluate(event_json)
        
        # Parse and return result
        return json.loads(result_json)
    
    async def eval_async(self, event: Dict[str, Any]) -> Dict[str, Any]:
        """
        Evaluate a hook event asynchronously
        
        Args:
            event: The hook event dictionary
        
        Returns:
            Decision dictionary with 'decision' and optional 'reason'
        
        Raises:
            RuntimeError: If engine not initialized
            ValueError: If event is invalid
        """
        if not self._engine:
            raise RuntimeError(
                "Cupcake engine not initialized. "
                "Call init_async() first."
            )
        
        # Convert event to JSON
        event_json = json.dumps(event)
        
        # Evaluate in thread pool (GIL released)
        result_json = await asyncio.to_thread(
            self._engine.evaluate, 
            event_json
        )
        
        # Parse and return result
        return json.loads(result_json)
    
    def is_ready(self) -> bool:
        """Check if the engine is initialized and ready"""
        return self._engine is not None and self._engine.is_ready()
    
    def version(self) -> str:
        """Get the engine version"""
        if not self._engine:
            return "Not initialized"
        return self._engine.version()
    
    def _resolve_path(self, path: str) -> str:
        """
        Resolve the project path
        
        Handles both relative and absolute paths, and expands user paths.
        """
        path_obj = Path(path).expanduser().resolve()
        
        # Check if path exists
        if not path_obj.exists():
            # Try to create it if it's a .cupcake directory
            if path_obj.name == ".cupcake":
                path_obj.mkdir(parents=True, exist_ok=True)
            else:
                raise RuntimeError(f"Path does not exist: {path_obj}")
        
        return str(path_obj)


# Create a default instance for module-level functions
_default_instance = Cupcake()

# Module-level convenience functions
def init(path: str = ".cupcake") -> None:
    """Initialize the default Cupcake instance"""
    _default_instance.init(path)

def eval(event: Dict[str, Any]) -> Dict[str, Any]:
    """Evaluate using the default Cupcake instance"""
    return _default_instance.eval(event)

async def init_async(path: str = ".cupcake") -> None:
    """Initialize the default Cupcake instance asynchronously"""
    await _default_instance.init_async(path)

async def eval_async(event: Dict[str, Any]) -> Dict[str, Any]:
    """Evaluate using the default Cupcake instance asynchronously"""
    return await _default_instance.eval_async(event)

def is_ready() -> bool:
    """Check if the default instance is ready"""
    return _default_instance.is_ready()

def version() -> str:
    """Get the version of the default instance"""
    return _default_instance.version()


# Export public API
__all__ = [
    "Cupcake",
    "init",
    "eval",
    "init_async", 
    "eval_async",
    "is_ready",
    "version",
    "OPA_VERSION",
]

# Package metadata
__version__ = "0.1.0"
__author__ = "Cupcake Contributors"