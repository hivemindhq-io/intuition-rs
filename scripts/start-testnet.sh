#!/bin/bash
# Start a SEPARATE Intuition TESTNET indexer stack (project: intuition-testnet).
# Safe to run alongside the mainnet `intuition` stack — different ports/volumes/DB.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
COMPOSE_PROJECT="intuition-testnet"

cd "$PROJECT_ROOT"

if [ -f "$PROJECT_ROOT/.env" ]; then
  # shellcheck disable=SC1091
  source "$PROJECT_ROOT/.env"
  export OPENAI_API_KEY PINATA_API_JWT PINATA_GATEWAY_TOKEN
  export BASE_MAINNET_RPC_URL BASE_SEPOLIA_RPC_URL
  export ETHEREUM_MAINNET_RPC_URL ETHEREUM_SEPOLIA_RPC_URL
  export LINEA_MAINNET_RPC_URL LINEA_SEPOLIA_RPC_URL
  export TRUST_MAINNET_RPC_URL LOCAL_INTUITION_RPC_URL POLYGON_AMOY_RPC_URL
fi

# Prefer public Intuition testnet RPC unless already set to a real endpoint
if [ -z "${TRUST_TESTNET_RPC_URL:-}" ] || [[ "$TRUST_TESTNET_RPC_URL" == *"reth"* ]] || [[ "$TRUST_TESTNET_RPC_URL" == *"example"* ]]; then
  # /http path matches hivemind-api's default and behaves better under load
  export TRUST_TESTNET_RPC_URL="https://testnet.rpc.intuition.systems/http"
fi
export TRUST_TESTNET_RPC_URL

export INDEXER_SCHEMA="trust_testnet"
export INTUITION_CONTRACT_ADDRESS="0x2Ece8D4dEdcB9918A398528f3fa4688b1d2CAB91"
export INITIAL_CONTRACT_VERSION="v2"

echo "Starting TESTNET shared services (project=$COMPOSE_PROJECT)..."
# Avoid `up --wait` across one-shot jobs (redis-setup / migrations) — it can hang.
docker compose -p "$COMPOSE_PROJECT" -f docker/docker-compose-shared-testnet.yml up \
  database pgai-installer vectorizer-worker redis redis-setup ipfs graphql-engine indexer-migrations -d

echo "Waiting for database + GraphQL..."
for i in $(seq 1 60); do
  if docker compose -p "$COMPOSE_PROJECT" -f docker/docker-compose-shared-testnet.yml exec -T database \
       pg_isready -U postgres -d storage >/dev/null 2>&1 \
     && curl -sf http://localhost:8082/healthz >/dev/null 2>&1; then
    break
  fi
  sleep 2
done
# Give indexer-migrations a moment to finish if still running
sleep 3

echo "Applying trust_testnet raw_data / histocrawler config..."
docker compose -p "$COMPOSE_PROJECT" -f docker/docker-compose-shared-testnet.yml exec -T database psql -U postgres -d storage \
  -f - < "$PROJECT_ROOT/infrastructure/indexer-and-cache-migrations/1740513400_create_trust_testnet_raw_data.up.sql"

# Prefer in-stack proxy; fall back to direct public RPC if you hit bandwidth limits
docker compose -p "$COMPOSE_PROJECT" -f docker/docker-compose-shared-testnet.yml exec -T database psql -U postgres -d storage -c \
  "UPDATE histocrawler.app_config
   SET rpc_url = 'http://testnet-rpc-proxy:3008/13579/proxy',
       contract_address = '0x2Ece8D4dEdcB9918A398528f3fa4688b1d2CAB91'
   WHERE indexer_schema = 'trust_testnet';"

if command -v hasura >/dev/null 2>&1; then
  echo "Applying local Hasura migrations to testnet GraphQL (localhost:8082)..."
  (
    cd "$PROJECT_ROOT/infrastructure/hasura"
    hasura metadata apply --endpoint http://localhost:8082 --admin-secret myadminsecretkey || true
    hasura migrate apply --database-name intuition --endpoint http://localhost:8082 --admin-secret myadminsecretkey --no-transaction
    hasura metadata apply --endpoint http://localhost:8082 --admin-secret myadminsecretkey || true
  )
else
  echo "Warning: hasura CLI not found; schema may be incomplete."
fi

echo "Starting TESTNET app services..."
docker compose -p "$COMPOSE_PROJECT" -f docker/docker-compose-apps-testnet.yml up \
  resolver_consumer ipfs_upload_consumer decoded_consumer api testnet-rpc-proxy histocrawler -d --force-recreate

echo ""
echo "=== TESTNET indexer up (separate from mainnet) ==="
echo "GraphQL:   http://localhost:8082/console"
echo "Postgres:  localhost:5436 (user/pass/db: postgres/postgres/storage)"
echo "Redis:     localhost:6380"
echo "RPC proxy: localhost:3028"
echo "Schema:    trust_testnet @ Multivault $INTUITION_CONTRACT_ADDRESS"
echo "RPC:       $TRUST_TESTNET_RPC_URL"
echo ""
echo "Mainnet stack (if running) is untouched: GraphQL :8080 / Postgres :5435"
echo ""
