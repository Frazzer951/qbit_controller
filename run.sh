#!/bin/bash

# Default sleep duration to 2 hours (120 minutes)
SLEEP_DURATION=${qbit_con_schedule:-120}

while true; do
  ./qbit_controller
  sleep "${SLEEP_DURATION}m"
done
