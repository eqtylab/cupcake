#!/bin/bash
# Script to remove test directory
if [ -d "/tmp/my-test-directory" ]; then
    rmdir /tmp/my-test-directory 2>/dev/null || rm -rf /tmp/my-test-directory
    echo "Test directory removed"
else
    echo "Test directory does not exist"
fi