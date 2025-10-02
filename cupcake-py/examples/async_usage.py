#!/usr/bin/env python3
"""
Async usage example for Cupcake Python bindings

This example demonstrates:
- Async initialization
- Concurrent evaluation
- Thread safety with asyncio
"""

import asyncio
import json
import sys
import time
from pathlib import Path

# Add parent directory to path for development
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

import cupcake


async def evaluate_event(event_id: int, event: dict) -> dict:
    """Evaluate a single event asynchronously"""
    start = time.time()
    
    try:
        result = await cupcake.eval_async(event)
        elapsed = time.time() - start
        
        return {
            "id": event_id,
            "success": True,
            "decision": result.get("decision"),
            "time_ms": elapsed * 1000,
            "result": result
        }
    except Exception as e:
        elapsed = time.time() - start
        return {
            "id": event_id,
            "success": False,
            "error": str(e),
            "time_ms": elapsed * 1000
        }


async def stress_test(num_events: int = 100):
    """Run a stress test with concurrent evaluations"""
    print(f"Starting stress test with {num_events} concurrent evaluations...")
    
    # Create test events
    events = [
        {
            "hookEventName": "PreToolUse",
            "tool_name": f"Tool_{i % 5}",
            "command": f"command_{i}",
            "request_id": i
        }
        for i in range(num_events)
    ]
    
    # Evaluate all events concurrently
    start = time.time()
    tasks = [evaluate_event(i, event) for i, event in enumerate(events)]
    results = await asyncio.gather(*tasks)
    total_time = time.time() - start
    
    # Analyze results
    successful = sum(1 for r in results if r["success"])
    failed = sum(1 for r in results if not r["success"])
    avg_time = sum(r["time_ms"] for r in results) / len(results)
    
    print(f"\n--- Stress Test Results ---")
    print(f"Total events: {num_events}")
    print(f"Successful: {successful}")
    print(f"Failed: {failed}")
    print(f"Total time: {total_time:.2f}s")
    print(f"Average time per event: {avg_time:.2f}ms")
    print(f"Events per second: {num_events / total_time:.1f}")
    
    # Show any errors
    errors = [r for r in results if not r["success"]]
    if errors:
        print(f"\nErrors encountered:")
        for err in errors[:5]:  # Show first 5 errors
            print(f"  Event {err['id']}: {err['error']}")


async def main():
    """Main async example"""
    
    # Initialize Cupcake asynchronously
    print("Initializing Cupcake engine asynchronously...")
    project_path = ".cupcake"
    
    try:
        await cupcake.init_async(project_path)
        print(f"✓ Engine initialized: {cupcake.version()}")
    except Exception as e:
        print(f"✗ Failed to initialize: {e}")
        return 1
    
    # Example 1: Simple async evaluation
    print("\n--- Example 1: Simple Async Evaluation ---")
    event = {
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "command": "echo 'Hello from async!'"
    }
    
    try:
        result = await cupcake.eval_async(event)
        print(f"Decision: {result.get('decision')}")
    except Exception as e:
        print(f"Error: {e}")
    
    # Example 2: Concurrent evaluations
    print("\n--- Example 2: Concurrent Evaluations ---")
    events = [
        {"hookEventName": "PreToolUse", "tool_name": "Bash", "id": 1},
        {"hookEventName": "SessionStart", "user": "alice", "id": 2},
        {"hookEventName": "PostToolUse", "tool_name": "Python", "id": 3},
    ]
    
    tasks = [cupcake.eval_async(e) for e in events]
    results = await asyncio.gather(*tasks, return_exceptions=True)
    
    for event, result in zip(events, results):
        if isinstance(result, Exception):
            print(f"Event {event['id']}: Error - {result}")
        else:
            print(f"Event {event['id']}: {result.get('decision', 'unknown')}")
    
    # Example 3: Stress test (optional)
    if "--stress" in sys.argv:
        await stress_test(100)
    
    print(f"\n✓ Engine ready: {cupcake.is_ready()}")
    return 0


if __name__ == "__main__":
    # Run the async main function
    exit_code = asyncio.run(main())
    sys.exit(exit_code)