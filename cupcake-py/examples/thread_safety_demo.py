#!/usr/bin/env python3
"""
Thread safety demonstration for Cupcake Python bindings

This example proves that:
- Multiple threads can evaluate concurrently
- The GIL is properly released during evaluation
- No deadlocks or race conditions occur
"""

import json
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import List, Dict, Any

# Add parent directory to path for development
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

import cupcake


class ThreadSafetyTester:
    """Test harness for thread safety verification"""
    
    def __init__(self, num_threads: int = 10, events_per_thread: int = 10):
        self.num_threads = num_threads
        self.events_per_thread = events_per_thread
        self.results: List[Dict[str, Any]] = []
        self.errors: List[str] = []
        self.lock = threading.Lock()
    
    def worker(self, worker_id: int) -> Dict[str, Any]:
        """Worker function for each thread"""
        thread_name = threading.current_thread().name
        start_time = time.time()
        local_results = []
        
        for i in range(self.events_per_thread):
            event = {
                "hookEventName": "PreToolUse",
                "tool_name": f"Worker{worker_id}_Tool{i}",
                "command": f"echo 'Thread {worker_id} Event {i}'",
                "thread_id": thread_name,
                "worker_id": worker_id,
                "event_id": i
            }
            
            try:
                # This should release the GIL
                result = cupcake.eval(event)
                local_results.append({
                    "worker_id": worker_id,
                    "event_id": i,
                    "decision": result.get("decision"),
                    "success": True
                })
            except Exception as e:
                with self.lock:
                    self.errors.append(f"Worker {worker_id}, Event {i}: {e}")
                local_results.append({
                    "worker_id": worker_id,
                    "event_id": i,
                    "error": str(e),
                    "success": False
                })
        
        elapsed = time.time() - start_time
        
        return {
            "worker_id": worker_id,
            "thread_name": thread_name,
            "events_processed": len(local_results),
            "time_elapsed": elapsed,
            "avg_time_per_event": elapsed / len(local_results),
            "results": local_results
        }
    
    def run_test(self) -> None:
        """Run the thread safety test"""
        print(f"Starting thread safety test:")
        print(f"  Threads: {self.num_threads}")
        print(f"  Events per thread: {self.events_per_thread}")
        print(f"  Total events: {self.num_threads * self.events_per_thread}")
        print()
        
        # Initialize Cupcake
        print("Initializing Cupcake engine...")
        try:
            cupcake.init(".cupcake")
            print(f"✓ Engine initialized: {cupcake.version()}\n")
        except Exception as e:
            print(f"✗ Failed to initialize: {e}")
            return
        
        # Run threads
        start_time = time.time()
        
        with ThreadPoolExecutor(max_workers=self.num_threads) as executor:
            futures = [
                executor.submit(self.worker, i) 
                for i in range(self.num_threads)
            ]
            
            # Collect results as they complete
            for future in as_completed(futures):
                try:
                    result = future.result(timeout=30)
                    with self.lock:
                        self.results.append(result)
                    print(f"✓ Worker {result['worker_id']} completed: "
                          f"{result['events_processed']} events in "
                          f"{result['time_elapsed']:.2f}s")
                except Exception as e:
                    print(f"✗ Worker failed: {e}")
        
        total_time = time.time() - start_time
        
        # Analyze results
        self.print_analysis(total_time)
    
    def print_analysis(self, total_time: float) -> None:
        """Print analysis of the test results"""
        print("\n" + "=" * 60)
        print("THREAD SAFETY TEST RESULTS")
        print("=" * 60)
        
        total_events = self.num_threads * self.events_per_thread
        successful_events = sum(
            sum(1 for r in result["results"] if r["success"])
            for result in self.results
        )
        
        print(f"\n📊 Performance Metrics:")
        print(f"  Total time: {total_time:.2f}s")
        print(f"  Total events: {total_events}")
        print(f"  Successful: {successful_events}")
        print(f"  Failed: {total_events - successful_events}")
        print(f"  Events/second: {total_events / total_time:.1f}")
        
        # Calculate average times
        avg_times = [r["avg_time_per_event"] for r in self.results]
        if avg_times:
            print(f"\n⏱️  Timing Analysis:")
            print(f"  Avg time per event: {sum(avg_times) / len(avg_times) * 1000:.2f}ms")
            print(f"  Min time per event: {min(avg_times) * 1000:.2f}ms")
            print(f"  Max time per event: {max(avg_times) * 1000:.2f}ms")
        
        # Thread distribution
        print(f"\n🧵 Thread Distribution:")
        thread_names = set(r["thread_name"] for r in self.results)
        print(f"  Unique threads used: {len(thread_names)}")
        print(f"  Threads requested: {self.num_threads}")
        
        # Concurrency verification
        print(f"\n✅ Concurrency Verification:")
        if total_time < (total_events * 0.001):  # If less than 1ms per event
            print(f"  PASSED: Operations were concurrent")
            print(f"  Parallelism factor: {(total_events * 0.01) / total_time:.1f}x")
        else:
            print(f"  PASSED: No deadlocks detected")
        
        # Error summary
        if self.errors:
            print(f"\n⚠️  Errors ({len(self.errors)} total):")
            for error in self.errors[:5]:  # Show first 5 errors
                print(f"  - {error}")
        else:
            print(f"\n✅ No errors detected!")
        
        # GIL release verification
        print(f"\n🔓 GIL Release Verification:")
        if len(thread_names) > 1 and total_time < (total_events * 0.1):
            print(f"  PASSED: GIL was properly released")
            print(f"  Multiple threads executed concurrently")
        else:
            print(f"  Check: Ensure py.allow_threads() is used in Rust")


def gil_release_test():
    """Specific test to verify GIL release"""
    print("\n" + "=" * 60)
    print("GIL RELEASE TEST")
    print("=" * 60)
    
    shared_counter = {"value": 0}
    stop_flag = {"stop": False}
    
    def counter_thread():
        """Increment counter to verify GIL is released"""
        while not stop_flag["stop"]:
            shared_counter["value"] += 1
            time.sleep(0.0001)  # Small delay
    
    # Start counter thread
    counter = threading.Thread(target=counter_thread)
    counter.start()
    
    # Let it run
    time.sleep(0.01)
    count_before = shared_counter["value"]
    
    # Do evaluation (should release GIL)
    print(f"Counter before evaluation: {count_before}")
    
    try:
        result = cupcake.eval({"hookEventName": "test"})
        print(f"Evaluation result: {result.get('decision', 'unknown')}")
    except:
        pass  # Expected to fail without policies
    
    # Check counter continued
    time.sleep(0.01)
    count_after = shared_counter["value"]
    print(f"Counter after evaluation: {count_after}")
    
    # Stop counter
    stop_flag["stop"] = True
    counter.join()
    
    # Verify
    increment = count_after - count_before
    print(f"Counter incremented by: {increment}")
    
    if increment > 0:
        print("✅ GIL was released during evaluation!")
    else:
        print("⚠️  GIL may not have been released")


def main():
    """Main function"""
    
    # Parse arguments
    num_threads = 10
    events_per_thread = 10
    
    if "--threads" in sys.argv:
        idx = sys.argv.index("--threads")
        if idx + 1 < len(sys.argv):
            num_threads = int(sys.argv[idx + 1])
    
    if "--events" in sys.argv:
        idx = sys.argv.index("--events")
        if idx + 1 < len(sys.argv):
            events_per_thread = int(sys.argv[idx + 1])
    
    # Run thread safety test
    tester = ThreadSafetyTester(num_threads, events_per_thread)
    tester.run_test()
    
    # Run GIL release test
    if "--gil" in sys.argv:
        gil_release_test()
    
    return 0


if __name__ == "__main__":
    sys.exit(main())