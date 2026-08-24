#!/bin/bash
# Stop the SEPARATE Intuition TESTNET indexer stack only (leaves mainnet `intuition` alone).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
COMPOSE_PROJECT="intuition-testnet"

cd "$PROJECT_ROOT"

# Dummy exports so compose file interpolation does not fail on down
export OPENAI_API_KEY="${OPENAI_API_KEY:-dummy}"
export PINATA_API_JWT="${PINATA_API_JWT:-dummy}"
export PINATA_GATEWAY_TOKEN="${PINATA_GATEWAY_TOKEN:-dummy}"
export BASE_MAINNET_RPC_URL="${BASE_MAINNET_RPC_URL:-dummy}"
export BASE_SEPOLIA_RPC_URL="${BASE_SEPOLIA_RPC_URL:-dummy}"
export ETHEREUM_MAINNET_RPC_URL="${ETHEREUM_MAINNET_RPC_URL:-dummy}"
export ETHEREUM_SEPOLIA_RPC_URL="${ETHEREUM_SEPOLIA_RPC_URL:-dummy}"
export LINEA_MAINNET_RPC_URL="${LINEA_MAINNET_RPC_URL:-dummy}"
export LINEA_SEPOLIA_RPC_URL="${LINEA_SEPOLIA_RPC_URL:-dummy}"
export TRUST_TESTNET_RPC_URL="${TRUST_TESTNET_RPC_URL:-dummy}"
export TRUST_MAINNET_RPC_URL="${TRUST_MAINNET_RPC_URL:-dummy}"
export LOCAL_INTUITION_RPC_URL="${LOCAL_INTUITION_RPC_URL:-dummy}"
export POLYGON_AMOY_RPC_URL="${POLYGON_AMOY_RPC_URL:-dummy}"
export INDEXER_SCHEMA="trust_testnet"
export INTUITION_CONTRACT_ADDRESS="0x2Ece8D4dEdcB9918A398528f3fa4688b1d2CAB91"
export INITIAL_CONTRACT_VERSION="v2"

docker compose -p "$COMPOSE_PROJECT" -f docker/docker-compose-apps-testnet.yml down || true
docker compose -p "$COMPOSE_PROJECT" -f docker/docker-compose-shared-testnet.yml down || true

echo "Stopped intuition-testnet stack. Mainnet (intuition) was not touched."
echo "To also wipe testnet volumes: docker compose -p intuition-testnet -f docker/docker-compose-shared-testnet.yml down -v"
