#!/bin/bash
# Run throttled histocrawlers on the HOST (bypasses broken Docker egress).
# Requires local DBs published on :5435 (mainnet) and :5436 (testnet).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BIN="$PROJECT_ROOT/target/release/histocrawler"

if [ ! -x "$BIN" ]; then
  echo "Missing $BIN — build with:"
  echo "  cd $PROJECT_ROOT && SQLX_OFFLINE=true cargo build --release -p histocrawler"
  exit 1
fi

cmd="${1:-status}"

start_one() {
  local name="$1"
  local db_url="$2"
  local schema="$3"
  local logfile="$4"
  shift 4
  local pidfile="/tmp/histocrawler-$name.pid"

  if [ -f "$pidfile" ] && kill -0 "$(cat "$pidfile")" 2>/dev/null; then
    echo "$name already running (pid $(cat "$pidfile"))"
    return
  fi

  nohup env \
    HISTOCRAWLER_DATABASE_URL="$db_url" \
    INDEXER_SCHEMA="$schema" \
    RUST_LOG=info \
    "$@" \
    "$BIN" >"$logfile" 2>&1 &
  echo $! >"$pidfile"
  echo "started $name pid=$! log=$logfile"
}

stop_one() {
  local name="$1"
  local pidfile="/tmp/histocrawler-$name.pid"
  if [ -f "$pidfile" ]; then
    local pid
    pid="$(cat "$pidfile")"
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      echo "stopped $name (pid $pid)"
    else
      echo "$name pidfile stale"
    fi
    rm -f "$pidfile"
  else
    echo "$name not running (no pidfile)"
  fi
}

case "$cmd" in
  start)
    docker update --restart=no testnet-histocrawler histocrawler 2>/dev/null || true
    docker stop testnet-histocrawler histocrawler 2>/dev/null || true

    start_one testnet \
      'postgres://postgres:postgres@127.0.0.1:5436/storage' \
      trust_testnet \
      /tmp/histocrawler-testnet.log \
      HISTOCRAWLER_MAX_BATCH_SIZE=80 \
      HISTOCRAWLER_MAX_LOGS_PER_REQUEST=400 \
      HISTOCRAWLER_INTER_BATCH_DELAY_MS=2500 \
      HISTOCRAWLER_TIMESTAMP_DELAY_MS=75

    start_one mainnet \
      'postgres://postgres:postgres@127.0.0.1:5435/storage' \
      trust_mainnet \
      /tmp/histocrawler-mainnet.log \
      HISTOCRAWLER_MAX_BATCH_SIZE=200 \
      HISTOCRAWLER_MAX_LOGS_PER_REQUEST=1500 \
      HISTOCRAWLER_INTER_BATCH_DELAY_MS=500 \
      HISTOCRAWLER_TIMESTAMP_DELAY_MS=20
    ;;
  stop)
    stop_one testnet
    stop_one mainnet
    ;;
  status)
    echo "processes:"
    pgrep -lf "$BIN" || echo "  (none)"
    echo
    echo "--- testnet log (tail) ---"
    tail -8 /tmp/histocrawler-testnet.log 2>/dev/null || echo "(no log)"
    echo "--- mainnet log (tail) ---"
    tail -8 /tmp/histocrawler-mainnet.log 2>/dev/null || echo "(no log)"
    ;;
  *)
    echo "Usage: $0 {start|stop|status}"
    exit 1
    ;;
esac
