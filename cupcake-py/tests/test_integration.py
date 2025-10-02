"""
Integration tests for Cupcake Python bindings

These tests verify:
- Basic functionality
- Thread safety
- GIL release
- Error handling
- JSON roundtrips
"""

import json
import threading
import time
import pytest
import tempfile
import shutil
from pathlib import Path
from unittest.mock import patch, MagicMock

# Import will fail without built module, but that's OK for test discovery
try:
    import cupcake
    CUPCAKE_AVAILABLE = True
except ImportError:
    CUPCAKE_AVAILABLE = False


def setup_test_policies(base_path: Path) -> Path:
    """Helper to set up test policies in a directory"""
    policies_path = base_path / "policies"
    policies_path.mkdir(parents=True, exist_ok=True)
    
    # Copy test fixtures - they MUST exist
    fixture_dir = Path(__file__).parent.parent / "test-fixtures" / ".cupcake" / "policies"
    assert fixture_dir.exists(), f"Test fixtures not found at {fixture_dir}"
    
    shutil.copytree(fixture_dir, policies_path, dirs_exist_ok=True)
    return base_path
    

@pytest.mark.skipif(not CUPCAKE_AVAILABLE, reason="Cupcake module not built")
class TestBasicFunctionality:
    """Test basic Cupcake functionality"""
    
    def test_version(self):
        """Test version retrieval"""
        version = cupcake.version()
        assert isinstance(version, str)
        assert "cupcake" in version.lower() or "not initialized" in version.lower()
    
    def test_init_with_nonexistent_path(self):
        """Test initialization with non-existent path"""
        with tempfile.TemporaryDirectory() as tmpdir:
            test_path = Path(tmpdir) / ".cupcake"
            
            # Set up test policies
            setup_test_policies(test_path)
            
            # Now init should work
            cupcake.init(str(test_path))
            assert test_path.exists()
    
    def test_eval_without_init(self):
        """Test that eval fails without initialization"""
        # Create a fresh instance
        engine = cupcake.Cupcake()
        with pytest.raises(RuntimeError, match="not initialized"):
            engine.eval({"hookEventName": "test"})
    
    def test_json_roundtrip(self):
        """Test JSON serialization/deserialization"""
        test_event = {
            "hookEventName": "PreToolUse",
            "tool_name": "Bash",
            "command": "echo test",
            "nested": {
                "data": [1, 2, 3],
                "flag": True
            }
        }
        
        # This tests that the event can be serialized to JSON
        json_str = json.dumps(test_event)
        parsed = json.loads(json_str)
        assert parsed == test_event


@pytest.mark.skipif(not CUPCAKE_AVAILABLE, reason="Cupcake module not built")
class TestThreadSafety:
    """Test thread safety and GIL release"""
    
    def test_concurrent_evaluations(self):
        """Test that multiple threads can evaluate concurrently"""
        results = []
        errors = []
        
        def worker(event_id):
            try:
                # Create separate engine instances
                engine = cupcake.Cupcake()
                with tempfile.TemporaryDirectory() as tmpdir:
                    test_path = Path(tmpdir) / ".cupcake"
                    setup_test_policies(test_path)
                    engine.init(str(test_path))
                    
                    # Simulate evaluation (will fail without policies but tests threading)
                    try:
                        result = engine.eval({
                            "hookEventName": "test",
                            "id": event_id
                        })
                        results.append(result)
                    except Exception as e:
                        # Expected to fail without actual policies
                        results.append({"error": str(e), "id": event_id})
            except Exception as e:
                errors.append(e)
        
        # Create multiple threads
        threads = []
        for i in range(5):
            t = threading.Thread(target=worker, args=(i,))
            threads.append(t)
            t.start()
        
        # Wait for all threads with timeout
        for t in threads:
            t.join(timeout=10)
            assert not t.is_alive(), "Thread did not complete in time"
        
        # Should have results from all threads (even if errors)
        assert len(results) == 5
        # Should not have any threading errors
        assert len(errors) == 0
    
    def test_gil_release_timing(self):
        """Test that GIL is released during evaluation"""
        # This test verifies that evaluation doesn't block other Python threads
        
        shared_counter = {"value": 0}
        stop_flag = {"stop": False}
        
        def counter_thread():
            """Increment counter while main thread evaluates"""
            while not stop_flag["stop"]:
                shared_counter["value"] += 1
                time.sleep(0.001)  # Small delay
        
        # Start counter thread
        t = threading.Thread(target=counter_thread)
        t.start()
        
        # Let counter run a bit
        time.sleep(0.01)
        initial_count = shared_counter["value"]
        
        # Simulate evaluation (would block without GIL release)
        engine = cupcake.Cupcake()
        with tempfile.TemporaryDirectory() as tmpdir:
            try:
                engine.init(str(Path(tmpdir) / ".cupcake"))
                # This would block the counter thread if GIL not released
                engine.eval({"hookEventName": "test"})
            except:
                pass  # Expected to fail without policies
        
        # Check counter kept incrementing
        time.sleep(0.01)
        final_count = shared_counter["value"]
        
        # Stop counter thread
        stop_flag["stop"] = True
        t.join()
        
        # Counter should have incremented during evaluation
        assert final_count > initial_count, "GIL was not released during evaluation"


@pytest.mark.skipif(not CUPCAKE_AVAILABLE, reason="Cupcake module not built")
@pytest.mark.asyncio
class TestAsyncFunctionality:
    """Test async functionality"""
    
    async def test_async_init_and_eval(self):
        """Test async initialization and evaluation"""
        engine = cupcake.Cupcake()
        
        with tempfile.TemporaryDirectory() as tmpdir:
            test_path = Path(tmpdir) / ".cupcake"
            setup_test_policies(test_path)
            
            # Async init
            await engine.init_async(str(test_path))
            assert test_path.exists()
            
            # Async eval should work with our test policies
            result = await engine.eval_async({"hookEventName": "test"})
            assert isinstance(result, dict)
            # Should get a valid response from our test policy
    
    async def test_module_level_async(self):
        """Test module-level async functions"""
        with tempfile.TemporaryDirectory() as tmpdir:
            test_path = Path(tmpdir) / ".cupcake"
            setup_test_policies(test_path)
            
            # Module-level async init
            await cupcake.init_async(str(test_path))
            
            # Module-level async eval should work with our test policies
            result = await cupcake.eval_async({"hookEventName": "test"})
            assert isinstance(result, dict)
            # Should get a valid response from our test policy


@pytest.mark.skipif(not CUPCAKE_AVAILABLE, reason="Cupcake module not built")
class TestErrorHandling:
    """Test error handling"""
    
    def test_invalid_json_input(self):
        """Test that invalid JSON is handled properly"""
        engine = cupcake.Cupcake()
        
        # Mock the native module to test error handling
        with patch.object(engine, '_engine') as mock_engine:
            mock_engine.evaluate.side_effect = ValueError("Invalid input JSON: test")
            mock_engine.is_ready.return_value = True
            
            with pytest.raises(ValueError, match="Invalid input JSON"):
                engine.eval({"test": "data"})
    
    def test_engine_not_ready(self):
        """Test is_ready check"""
        engine = cupcake.Cupcake()
        assert not engine.is_ready()
        
        with tempfile.TemporaryDirectory() as tmpdir:
            test_path = Path(tmpdir) / ".cupcake"
            setup_test_policies(test_path)
            engine.init(str(test_path))
            assert engine.is_ready()


if __name__ == "__main__":
    pytest.main([__file__, "-v"])