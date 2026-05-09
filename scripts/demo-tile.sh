#!/usr/bin/env bash
# Emits a labeled ticking output for the streeem demo. Usage: demo-tile.sh <label>
set -eu
LABEL="${1:-tile}"
i=0
while true; do
  echo "${LABEL} tick #${i} $(date +%T)"
  i=$((i+1))
  sleep 1
done
