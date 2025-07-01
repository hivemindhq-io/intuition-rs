-- SurrealDB Schema for Intuition Protocol
-- Hybrid Document-Graph Model for Semantic Knowledge Graph with Prediction Markets

-- ================================
-- NAMESPACE AND DATABASE SETUP
-- ================================

-- Use this for multi-tenant support
-- DEFINE NAMESPACE intuition;
-- USE NS intuition;
-- DEFINE DATABASE mainnet;
-- USE DB mainnet;

-- ================================
-- CORE DOCUMENT TABLES
-- ================================

-- Atoms: Core semantic units (nodes in knowledge graph)
DEFINE TABLE atom SCHEMAFULL;

DEFINE FIELD term_id ON TABLE atom TYPE number;
DEFINE FIELD wallet_id ON TABLE atom TYPE string;
DEFINE FIELD creator_id ON TABLE atom TYPE record<account>;
DEFINE FIELD atom_type ON TABLE atom TYPE string;
DEFINE FIELD label ON TABLE atom TYPE option<string>;
DEFINE FIELD emoji ON TABLE atom TYPE option<string>;
DEFINE FIELD image ON TABLE atom TYPE option<string>;
DEFINE FIELD data ON TABLE atom TYPE option<object>;
DEFINE FIELD raw_data ON TABLE atom TYPE string;
DEFINE FIELD value_reference ON TABLE atom TYPE option<object>;
DEFINE FIELD block_number ON TABLE atom TYPE number;
DEFINE FIELD transaction_hash ON TABLE atom TYPE string;
DEFINE FIELD resolving_status ON TABLE atom TYPE string DEFAULT "pending";
DEFINE FIELD log_index ON TABLE atom TYPE number;
DEFINE FIELD created_at ON TABLE atom TYPE datetime DEFAULT time::now();

-- Accounts: User/wallet information
DEFINE TABLE account SCHEMAFULL;

DEFINE FIELD id ON TABLE account TYPE string;
DEFINE FIELD label ON TABLE account TYPE string;
DEFINE FIELD image ON TABLE account TYPE option<string>;
DEFINE FIELD account_type ON TABLE account TYPE string DEFAULT "default";
DEFINE FIELD atom_id ON TABLE account TYPE option<record<atom>>;
DEFINE FIELD created_at ON TABLE account TYPE datetime DEFAULT time::now();

-- Vaults: Investment containers for atoms/triples
DEFINE TABLE vault SCHEMAFULL;

DEFINE FIELD term_id ON TABLE vault TYPE record<atom>;
DEFINE FIELD curve_id ON TABLE vault TYPE number;
DEFINE FIELD total_shares ON TABLE vault TYPE number DEFAULT 0;
DEFINE FIELD current_share_price ON TABLE vault TYPE number DEFAULT 0;
DEFINE FIELD position_count ON TABLE vault TYPE number DEFAULT 0;
DEFINE FIELD total_assets ON TABLE vault TYPE number DEFAULT 0;
DEFINE FIELD market_cap ON TABLE vault TYPE number DEFAULT 0;
DEFINE FIELD block_number ON TABLE vault TYPE number;
DEFINE FIELD transaction_hash ON TABLE vault TYPE string;
DEFINE FIELD log_index ON TABLE vault TYPE number;
DEFINE FIELD created_at ON TABLE vault TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON TABLE vault TYPE datetime DEFAULT time::now();

-- ================================
-- GRAPH RELATIONSHIP TABLES
-- ================================

-- Triples: Subject-Predicate-Object semantic relationships
DEFINE TABLE triple SCHEMAFULL;

DEFINE FIELD in ON TABLE triple TYPE record<atom>;
DEFINE FIELD out ON TABLE triple TYPE record<atom>;
DEFINE FIELD predicate_id ON TABLE triple TYPE record<atom>;
DEFINE FIELD creator_id ON TABLE triple TYPE record<account>;
DEFINE FIELD counter_term_id ON TABLE triple TYPE option<number>;
DEFINE FIELD block_number ON TABLE triple TYPE number;
DEFINE FIELD transaction_hash ON TABLE triple TYPE string;
DEFINE FIELD log_index ON TABLE triple TYPE number;
DEFINE FIELD created_at ON TABLE triple TYPE datetime DEFAULT time::now();

-- Positions: User stakes in atoms/triples (financial relationships)
DEFINE TABLE position SCHEMAFULL;

DEFINE FIELD in ON TABLE position TYPE record<account>;
DEFINE FIELD out ON TABLE position TYPE record<atom>;
DEFINE FIELD shares ON TABLE position TYPE number;
DEFINE FIELD curve_id ON TABLE position TYPE number;
DEFINE FIELD entry_price ON TABLE position TYPE number;
DEFINE FIELD block_number ON TABLE position TYPE number;
DEFINE FIELD transaction_hash ON TABLE position TYPE string;
DEFINE FIELD transaction_index ON TABLE position TYPE number;
DEFINE FIELD log_index ON TABLE position TYPE number;
DEFINE FIELD created_at ON TABLE position TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON TABLE position TYPE datetime DEFAULT time::now();

-- Claims: Account claims on specific positions
DEFINE TABLE claim SCHEMAFULL;

DEFINE FIELD in ON TABLE claim TYPE record<account>;
DEFINE FIELD out ON TABLE claim TYPE record<position>;
DEFINE FIELD claim_type ON TABLE claim TYPE string;
DEFINE FIELD metadata ON TABLE claim TYPE option<object>;
DEFINE FIELD block_number ON TABLE claim TYPE number;
DEFINE FIELD transaction_hash ON TABLE claim TYPE string;
DEFINE FIELD created_at ON TABLE claim TYPE datetime DEFAULT time::now();

-- Deposits: Investment transactions (account -> vault relationships)
DEFINE TABLE deposit SCHEMAFULL;

DEFINE FIELD in ON TABLE deposit TYPE record<account>;
DEFINE FIELD out ON TABLE deposit TYPE record<vault>;
DEFINE FIELD shares_for_receiver ON TABLE deposit TYPE number;
DEFINE FIELD assets_for_receiver ON TABLE deposit TYPE number;
DEFINE FIELD entry_fee ON TABLE deposit TYPE number;
DEFINE FIELD receiver_total_shares_in_vault ON TABLE deposit TYPE number;
DEFINE FIELD sender_assets_after_total_fees ON TABLE deposit TYPE number;
DEFINE FIELD block_number ON TABLE deposit TYPE number;
DEFINE FIELD transaction_hash ON TABLE deposit TYPE string;
DEFINE FIELD log_index ON TABLE deposit TYPE number;
DEFINE FIELD created_at ON TABLE deposit TYPE datetime DEFAULT time::now();

-- Redemptions: Withdrawal transactions (account <- vault relationships)
DEFINE TABLE redemption SCHEMAFULL;

DEFINE FIELD in ON TABLE redemption TYPE record<account>;
DEFINE FIELD out ON TABLE redemption TYPE record<vault>;
DEFINE FIELD shares_redeemed ON TABLE redemption TYPE number;
DEFINE FIELD assets_for_receiver ON TABLE redemption TYPE number;
DEFINE FIELD exit_fee ON TABLE redemption TYPE number;
DEFINE FIELD sender_total_shares_in_vault ON TABLE redemption TYPE number;
DEFINE FIELD receiver_assets_after_total_fees ON TABLE redemption TYPE number;
DEFINE FIELD block_number ON TABLE redemption TYPE number;
DEFINE FIELD transaction_hash ON TABLE redemption TYPE string;
DEFINE FIELD log_index ON TABLE redemption TYPE number;
DEFINE FIELD created_at ON TABLE redemption TYPE datetime DEFAULT time::now();

-- ================================
-- POLYMORPHIC CONTENT TABLES
-- ================================

-- Person: Individual person entities
DEFINE TABLE person SCHEMAFULL;

DEFINE FIELD identifier ON TABLE person TYPE string;
DEFINE FIELD name ON TABLE person TYPE string;
DEFINE FIELD description ON TABLE person TYPE option<string>;
DEFINE FIELD image ON TABLE person TYPE option<string>;
DEFINE FIELD url ON TABLE person TYPE option<string>;
DEFINE FIELD email ON TABLE person TYPE option<string>;
DEFINE FIELD social_links ON TABLE person TYPE option<object>;
DEFINE FIELD created_at ON TABLE person TYPE datetime DEFAULT time::now();

-- Organization: Company, DAO, or institutional entities
DEFINE TABLE organization SCHEMAFULL;

DEFINE FIELD name ON TABLE organization TYPE string;
DEFINE FIELD description ON TABLE organization TYPE option<string>;
DEFINE FIELD image ON TABLE organization TYPE option<string>;
DEFINE FIELD url ON TABLE organization TYPE option<string>;
DEFINE FIELD email ON TABLE organization TYPE option<string>;
DEFINE FIELD organization_type ON TABLE organization TYPE option<string>;
DEFINE FIELD founded_date ON TABLE organization TYPE option<datetime>;
DEFINE FIELD created_at ON TABLE organization TYPE datetime DEFAULT time::now();

-- Thing: General concepts, objects, or entities
DEFINE TABLE thing SCHEMAFULL;

DEFINE FIELD name ON TABLE thing TYPE string;
DEFINE FIELD description ON TABLE thing TYPE option<string>;
DEFINE FIELD image ON TABLE thing TYPE option<string>;
DEFINE FIELD url ON TABLE thing TYPE option<string>;
DEFINE FIELD category ON TABLE thing TYPE option<string>;
DEFINE FIELD tags ON TABLE thing TYPE option<array<string>>;
DEFINE FIELD created_at ON TABLE thing TYPE datetime DEFAULT time::now();

-- Book: Publication entities
DEFINE TABLE book SCHEMAFULL;

DEFINE FIELD name ON TABLE book TYPE string;
DEFINE FIELD description ON TABLE book TYPE option<string>;
DEFINE FIELD genre ON TABLE book TYPE option<string>;
DEFINE FIELD url ON TABLE book TYPE option<string>;
DEFINE FIELD author ON TABLE book TYPE option<string>;
DEFINE FIELD isbn ON TABLE book TYPE option<string>;
DEFINE FIELD publication_date ON TABLE book TYPE option<datetime>;
DEFINE FIELD created_at ON TABLE book TYPE datetime DEFAULT time::now();

-- Text Object: Plain text content
DEFINE TABLE text_object SCHEMAFULL;

DEFINE FIELD data ON TABLE text_object TYPE string;
DEFINE FIELD encoding ON TABLE text_object TYPE string DEFAULT "utf-8";
DEFINE FIELD language ON TABLE text_object TYPE option<string>;
DEFINE FIELD created_at ON TABLE text_object TYPE datetime DEFAULT time::now();

-- JSON Object: Structured JSON content
DEFINE TABLE json_object SCHEMAFULL;

DEFINE FIELD data ON TABLE json_object TYPE object;
DEFINE FIELD schema_version ON TABLE json_object TYPE option<string>;
DEFINE FIELD created_at ON TABLE json_object TYPE datetime DEFAULT time::now();

-- Byte Object: Binary content
DEFINE TABLE byte_object SCHEMAFULL;

DEFINE FIELD data ON TABLE byte_object TYPE bytes;
DEFINE FIELD content_type ON TABLE byte_object TYPE option<string>;
DEFINE FIELD encoding ON TABLE byte_object TYPE option<string>;
DEFINE FIELD created_at ON TABLE byte_object TYPE datetime DEFAULT time::now();

-- CAIP-10: Blockchain account identifiers
DEFINE TABLE caip10 SCHEMAFULL;

DEFINE FIELD namespace ON TABLE caip10 TYPE string;
DEFINE FIELD chain_id ON TABLE caip10 TYPE string;
DEFINE FIELD account_address ON TABLE caip10 TYPE string;
DEFINE FIELD created_at ON TABLE caip10 TYPE datetime DEFAULT time::now();

-- ================================
-- INDEXES FOR PERFORMANCE
-- ================================

-- Core entity indexes
DEFINE INDEX idx_atom_term_id ON TABLE atom COLUMNS term_id UNIQUE;
DEFINE INDEX idx_atom_type ON TABLE atom COLUMNS atom_type;
DEFINE INDEX idx_atom_creator ON TABLE atom COLUMNS creator_id;
DEFINE INDEX idx_atom_block ON TABLE atom COLUMNS block_number;
DEFINE INDEX idx_atom_status ON TABLE atom COLUMNS resolving_status;

DEFINE INDEX idx_account_id ON TABLE account COLUMNS id UNIQUE;
DEFINE INDEX idx_account_type ON TABLE account COLUMNS account_type;

DEFINE INDEX idx_vault_term ON TABLE vault COLUMNS term_id UNIQUE;
DEFINE INDEX idx_vault_market_cap ON TABLE vault COLUMNS market_cap;

-- Graph relationship indexes
DEFINE INDEX idx_triple_subject ON TABLE triple COLUMNS in;
DEFINE INDEX idx_triple_predicate ON TABLE triple COLUMNS predicate_id;
DEFINE INDEX idx_triple_object ON TABLE triple COLUMNS out;
DEFINE INDEX idx_triple_spo ON TABLE triple COLUMNS in, predicate_id, out UNIQUE;

DEFINE INDEX idx_position_account ON TABLE position COLUMNS in;
DEFINE INDEX idx_position_atom ON TABLE position COLUMNS out;
DEFINE INDEX idx_position_account_atom ON TABLE position COLUMNS in, out UNIQUE;
DEFINE INDEX idx_position_shares ON TABLE position COLUMNS shares;

DEFINE INDEX idx_claim_account ON TABLE claim COLUMNS in;
DEFINE INDEX idx_claim_position ON TABLE claim COLUMNS out;

-- Financial transaction indexes
DEFINE INDEX idx_deposit_account ON TABLE deposit COLUMNS in;
DEFINE INDEX idx_deposit_vault ON TABLE deposit COLUMNS out;
DEFINE INDEX idx_deposit_created_at ON TABLE deposit COLUMNS created_at;

DEFINE INDEX idx_redemption_account ON TABLE redemption COLUMNS in;
DEFINE INDEX idx_redemption_vault ON TABLE redemption COLUMNS out;
DEFINE INDEX idx_redemption_created_at ON TABLE redemption COLUMNS created_at;

-- Composite indexes for complex queries
DEFINE INDEX idx_position_value ON TABLE position COLUMNS shares, created_at;

-- Content type indexes
DEFINE INDEX idx_person_identifier ON TABLE person COLUMNS identifier;
DEFINE INDEX idx_organization_name ON TABLE organization COLUMNS name;
DEFINE INDEX idx_thing_category ON TABLE thing COLUMNS category;
DEFINE INDEX idx_book_author ON TABLE book COLUMNS author;

-- ================================
-- ADDITIONAL CONSTRAINTS
-- ================================

-- Ensure unique positions per account-atom pair
DEFINE INDEX idx_unique_position ON TABLE position COLUMNS in, out UNIQUE;

-- Ensure unique triple relationships
DEFINE INDEX idx_unique_triple ON TABLE triple COLUMNS in, predicate_id, out UNIQUE;

-- Ensure valid account references
DEFINE INDEX idx_atom_creator_ref ON TABLE atom COLUMNS creator_id;
DEFINE INDEX idx_triple_creator_ref ON TABLE triple COLUMNS creator_id;

-- Time-based partitioning hint for analytics
DEFINE INDEX idx_position_temporal ON TABLE position COLUMNS created_at, block_number;
DEFINE INDEX idx_deposit_temporal ON TABLE deposit COLUMNS created_at, block_number;
DEFINE INDEX idx_redemption_temporal ON TABLE redemption COLUMNS created_at, block_number;

-- ================================
-- SCHEMA COMPLETE
-- ================================

-- Note: Custom functions can be added later once the core schema is working
-- Basic schema is now ready for use with SurrealDB
