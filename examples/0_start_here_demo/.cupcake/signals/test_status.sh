#!/bin/bash
# Structured signal: Test execution status with rich data
# This demonstrates complex JSON output that policies can access deeply
echo '{
  "passing": false,
  "coverage": 87.5,
  "duration": 12.3,
  "last_run": "2025-08-14T10:30:00Z",
  "failed_tests": [
    "test_auth.py::test_login",
    "test_db.py::test_connection"
  ],
  "environment": "ci"
}'