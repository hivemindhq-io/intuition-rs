CREATE SCHEMA IF NOT EXISTS trust_mainnet;

CREATE TABLE IF NOT EXISTS trust_mainnet.raw_data (
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

CREATE OR REPLACE FUNCTION trust_mainnet.notify_raw_logs()
RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify('trust_mainnet_raw_logs_channel', row_to_json(NEW)::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trust_mainnet_raw_logs_notify_trigger
    AFTER INSERT ON trust_mainnet.raw_data
    FOR EACH ROW
    EXECUTE FUNCTION trust_mainnet.notify_raw_logs();
    
CREATE INDEX idx_trust_mainnet_raw_data_block_number ON trust_mainnet.raw_data(block_number);
CREATE INDEX idx_trust_mainnet_raw_data_block_timestamp ON trust_mainnet.raw_data(block_timestamp);
CREATE INDEX idx_trust_mainnet_raw_data_transaction_hash ON trust_mainnet.raw_data(transaction_hash);
CREATE INDEX idx_trust_mainnet_raw_data_address ON trust_mainnet.raw_data(address);
CREATE INDEX idx_trust_mainnet_raw_data_topics ON trust_mainnet.raw_data(topics);

INSERT INTO histocrawler.app_config (indexer_schema, rpc_url, start_block, end_block, contract_address, raw_logs_channel)
VALUES ('trust_mainnet', 'http://prod-rpc-proxy:3008/1155/proxy', 1311, NULL, '0x6E35cF57A41fA15eA0EaE9C33e751b01A784Fe7e', 'trust_mainnet_raw_logs_channel');