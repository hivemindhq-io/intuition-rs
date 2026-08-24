DELETE FROM histocrawler.app_config WHERE indexer_schema = 'trust_testnet';
DROP TRIGGER IF EXISTS trust_testnet_raw_logs_notify_trigger ON trust_testnet.raw_data;
DROP FUNCTION IF EXISTS trust_testnet.notify_raw_logs();
DROP TABLE IF EXISTS trust_testnet.raw_data;
DROP SCHEMA IF EXISTS trust_testnet;
