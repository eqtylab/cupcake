#!/bin/bash
# Action for dangerous rm command
echo "[$(date)] BLOCKED: Dangerous rm command prevented - $@" >> /tmp/cupcake_audit.log