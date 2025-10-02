#!/usr/bin/env python3
"""
Basic usage example for Cupcake Python bindings

This example demonstrates:
- Initialization
- Synchronous evaluation
- Decision handling
"""

import json
import sys
from pathlib import Path

# Add parent directory to path for development
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

import cupcake


def main():
    """Main example function"""
    
    # Initialize Cupcake with a project directory
    print("Initializing Cupcake engine...")
    project_path = ".cupcake"  # Or path to your policies
    
    try:
        cupcake.init(project_path)
        print(f"✓ Engine initialized: {cupcake.version()}")
    except Exception as e:
        print(f"✗ Failed to initialize: {e}")
        return 1
    
    # Example events to evaluate
    events = [
        {
            "hookEventName": "PreToolUse",
            "tool_name": "Bash",
            "command": "rm -rf /",
            "description": "Dangerous command that should be blocked"
        },
        {
            "hookEventName": "PreToolUse", 
            "tool_name": "Python",
            "code": "print('Hello, World!')",
            "description": "Safe Python code"
        },
        {
            "hookEventName": "SessionStart",
            "user": "alice",
            "timestamp": "2024-01-01T00:00:00Z",
            "description": "New session started"
        }
    ]
    
    # Evaluate each event
    for i, event in enumerate(events, 1):
        print(f"\n--- Event {i}: {event.get('description', 'No description')} ---")
        print(f"Event type: {event['hookEventName']}")
        
        if "tool_name" in event:
            print(f"Tool: {event['tool_name']}")
        
        try:
            # Evaluate the event
            result = cupcake.eval(event)
            
            # Handle the decision
            decision = result.get("decision", "unknown")
            print(f"Decision: {decision}")
            
            if decision in ["halt", "deny", "block"]:
                reason = result.get("reason", "No reason provided")
                print(f"⛔ Action blocked: {reason}")
            elif decision == "ask":
                reason = result.get("reason", "Confirmation required")
                print(f"❓ User confirmation needed: {reason}")
            elif decision == "allow":
                context = result.get("context", [])
                if context:
                    print(f"✅ Allowed with context: {', '.join(context)}")
                else:
                    print("✅ Allowed")
            
            # Show full result in debug mode
            if "--debug" in sys.argv:
                print(f"Full result: {json.dumps(result, indent=2)}")
                
        except Exception as e:
            print(f"✗ Evaluation failed: {e}")
    
    print("\n--- Summary ---")
    print(f"Evaluated {len(events)} events")
    print(f"Engine ready: {cupcake.is_ready()}")
    
    return 0


if __name__ == "__main__":
    sys.exit(main())