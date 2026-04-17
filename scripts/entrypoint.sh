#!/bin/sh
set -e

# Start the server in the background so we can seed against it.
/usr/local/bin/arx-grpc &
SERVER_PID=$!

# Forward SIGTERM/SIGINT to the server process so graceful shutdown works.
trap 'kill -TERM $SERVER_PID' TERM INT

# Wait until /health responds.
echo "entrypoint: waiting for server..."
until wget -qO- "http://localhost:${PORT:-50051}/health" >/dev/null 2>&1; do
  sleep 1
done
echo "entrypoint: server ready"

# Run seed exactly once. The marker file lives on the persistent volume (/data)
# so it survives container restarts but not volume resets.
if [ -n "$SEED_EMAIL" ] && [ ! -f /data/.seeded ]; then
  echo "entrypoint: seeding..."
  if python3 /usr/local/bin/seed.py; then
    touch /data/.seeded
    echo "entrypoint: seed complete"
  else
    echo "entrypoint: seed failed (server will still run)" >&2
  fi
elif [ -f /data/.seeded ]; then
  echo "entrypoint: already seeded, skipping"
fi

# Wait for the server; exit with its exit code.
wait $SERVER_PID
