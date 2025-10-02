#!/bin/bash
echo "[$(date)] BLOCKED: Attempt to write .txt file - $@" >> /tmp/cupcake_audit.log