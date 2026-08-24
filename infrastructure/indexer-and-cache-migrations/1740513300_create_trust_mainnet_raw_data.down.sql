DELETE FROM histocrawler.app_config WHERE indexer_schema = 'trust_mainnet';

DROP TRIGGER IF EXISTS trust_mainnet_raw_logs_notify_trigger ON trust_mainnet.raw_data;
DROP FUNCTION IF EXISTS trust_mainnet.notify_raw_logs();
DROP TABLE IF EXISTS trust_mainnet.raw_data;

DROP INDEX IF EXISTS idx_trust_mainnet_raw_data_block_number;
DROP INDEX IF EXISTS idx_trust_mainnet_raw_data_block_timestamp;
DROP INDEX IF EXISTS idx_trust_mainnet_raw_data_transaction_hash;
DROP INDEX IF EXISTS idx_trust_mainnet_raw_data_address;
DROP INDEX IF EXISTS idx_trust_mainnet_raw_data_topics;

DROP SCHEMA IF EXISTS trust_mainnet;
