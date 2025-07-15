#!/bin/bash

ADDRESS="127.0.0.1"
PORT="3000"
CONNECTIONS=100

echo "Starting load test with $CONNECTIONS connections using netcat (nc)..."

# Check if nc is installed
if ! command -v nc &>/dev/null; then
    echo "Error: netcat (nc) is not installed. Please install it to run this test."
    exit 1
fi

for i in $(seq 1 $CONNECTIONS); do
    # Use netcat to open a connection.
    # We pipe the output of 'sleep' into nc.
    # This keeps the connection open for 60 seconds.
    # When sleep finishes, the pipe closes, and nc exits.
    (sleep 60 | nc $ADDRESS $PORT >/dev/null 2>&1) &

    # A small sleep to avoid overwhelming the system instantly
    if ((i % 10 == 0)); then
        echo "Opened $i connections..."
        sleep 0.1
    fi
done

echo "All connection attempts have been made. Waiting for background jobs to complete..."
# Wait for all background jobs to finish
wait
echo "Test complete."
