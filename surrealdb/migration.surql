-- This is enables fast iteration in dev env
-- docker compose -f docker-compose-shared.yml up surrealdb-migrations

REMOVE TABLE IF EXISTS account;
REMOVE TABLE IF EXISTS atom;
REMOVE TABLE IF EXISTS triple;

DEFINE TABLE atom SCHEMAFULL;

DEFINE FIELD term_id ON TABLE atom TYPE string;
DEFINE FIELD atom_type ON TABLE atom TYPE string;
DEFINE FIELD label ON TABLE atom TYPE option<string>;
DEFINE FIELD data ON TABLE atom TYPE option<string>;

DEFINE TABLE account SCHEMAFULL;

DEFINE FIELD id ON TABLE account TYPE string;
DEFINE FIELD label ON TABLE account TYPE string;
DEFINE FIELD image ON TABLE account TYPE option<string>;
DEFINE FIELD account_type ON TABLE account TYPE string DEFAULT "default";
DEFINE FIELD atom_id ON TABLE account TYPE option<record<atom>>;
DEFINE FIELD created_at ON TABLE account TYPE datetime DEFAULT time::now();

DEFINE TABLE triple SCHEMAFULL TYPE RELATION;
DEFINE FIELD predicate ON TABLE triple TYPE record<atom>;

