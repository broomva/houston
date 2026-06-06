#!/usr/bin/env bash
# Entry point for the Houston Engine container.
#
# PaaS platforms (Railway, Heroku, Fly-with-$PORT) inject $PORT and route
# their public domain to whatever the process binds. Honor it so the SAME
# image deploys cleanly everywhere. docker-compose / systemd set HOUSTON_BIND
# explicitly and do NOT set $PORT, so they are unaffected.
set -euo pipefail

if [ -n "${PORT:-}" ]; then
  export HOUSTON_BIND="0.0.0.0:${PORT}"
  export HOUSTON_BIND_ALL=1
fi

exec /usr/local/bin/houston-engine "$@"
