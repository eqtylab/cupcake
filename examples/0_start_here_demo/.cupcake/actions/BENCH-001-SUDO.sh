#!/bin/bash
# Action for sudo usage
echo "[$(date)] BLOCKED: Sudo usage attempted - $@" >> /tmp/cupcake_audit.log