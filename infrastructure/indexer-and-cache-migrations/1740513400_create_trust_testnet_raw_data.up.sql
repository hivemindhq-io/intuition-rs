CREATE SCHEMA IF NOT EXISTS trust_testnet;

CREATE TABLE IF NOT EXISTS trust_testnet.raw_data (
    id SERIAL PRIMARY KEY NOT NULL,
    gs_id VARCHAR(200),
    block_number BIGINT,
    block_hash VARCHAR(200),
    transaction_hash VARCHAR(200),
    transaction_index BIGINT,
    log_index BIGINT,
    address VARCHAR(42),
    data TEXT,
    topics TEXT[],
    block_timestamp BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE OR REPLACE FUNCTION trust_testnet.notify_raw_logs()
RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify('trust_testnet_raw_logs_channel', row_to_json(NEW)::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trust_testnet_raw_logs_notify_trigger ON trust_testnet.raw_data;
CREATE TRIGGER trust_testnet_raw_logs_notify_trigger
    AFTER INSERT ON trust_testnet.raw_data
    FOR EACH ROW
    EXECUTE FUNCTION trust_testnet.notify_raw_logs();

CREATE INDEX IF NOT EXISTS idx_trust_testnet_raw_data_block_number ON trust_testnet.raw_data(block_number);
CREATE INDEX IF NOT EXISTS idx_trust_testnet_raw_data_block_timestamp ON trust_testnet.raw_data(block_timestamp);
CREATE INDEX IF NOT EXISTS idx_trust_testnet_raw_data_transaction_hash ON trust_testnet.raw_data(transaction_hash);
CREATE INDEX IF NOT EXISTS idx_trust_testnet_raw_data_address ON trust_testnet.raw_data(address);
CREATE INDEX IF NOT EXISTS idx_trust_testnet_raw_data_topics ON trust_testnet.raw_data(topics);

-- Official Intuition testnet Multivault (chain 13579). Deployed at block 8092570;
-- an eth_getLogs lookback window is NOT a safe way to find this — a capped window
-- makes the contract look far younger than it is and silently skips its history.
INSERT INTO histocrawler.app_config (indexer_schema, rpc_url, start_block, end_block, contract_address, raw_logs_channel)
VALUES (
  'trust_testnet',
  'http://testnet-rpc-proxy:3008/13579/proxy',
  8092570,
  NULL,
  '0x2Ece8D4dEdcB9918A398528f3fa4688b1d2CAB91',
  'trust_testnet_raw_logs_channel'
)
ON CONFLICT (indexer_schema) DO UPDATE SET
  rpc_url = EXCLUDED.rpc_url,
  contract_address = EXCLUDED.contract_address,
  raw_logs_channel = EXCLUDED.raw_logs_channel;

-- Only seed cursor on first insert (NULL last_processed_block)
UPDATE histocrawler.app_config
SET last_processed_block = 8092569
WHERE indexer_schema = 'trust_testnet' AND last_processed_block IS NULL;
