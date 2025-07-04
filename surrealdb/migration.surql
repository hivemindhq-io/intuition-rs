-- This is enables fast iteration in dev env
-- docker compose -f docker-compose-shared.yml up surrealdb-migrations

REMOVE TABLE IF EXISTS atom_value;
REMOVE TABLE IF EXISTS account;
REMOVE TABLE IF EXISTS atom;
REMOVE TABLE IF EXISTS triple;


DEFINE TABLE atom_value SCHEMALESS;

DEFINE TABLE atom SCHEMAFULL;

DEFINE FIELD term_id ON TABLE atom TYPE string;
DEFINE FIELD atom_type ON TABLE atom TYPE string;
DEFINE FIELD label ON TABLE atom TYPE option<string>;
DEFINE FIELD data ON TABLE atom TYPE option<string>;
DEFINE FIELD atom_value ON TABLE atom TYPE option<record<atom_value>>;

DEFINE TABLE account SCHEMAFULL;

DEFINE FIELD id ON TABLE account TYPE string;
DEFINE FIELD label ON TABLE account TYPE string;
DEFINE FIELD image ON TABLE account TYPE option<string>;
DEFINE FIELD account_type ON TABLE account TYPE string DEFAULT "default";
DEFINE FIELD atom_id ON TABLE account TYPE option<record<atom>>;
DEFINE FIELD created_at ON TABLE account TYPE datetime DEFAULT time::now();

DEFINE TABLE triple SCHEMAFULL TYPE RELATION;
DEFINE FIELD predicate ON TABLE triple TYPE record<atom>;

DEFINE FUNCTION OVERWRITE fn::get_patch_predicate_id() {
  let $res = select id from atom where data='json_patch';
  return $res[0].id;
};

DEFINE FUNCTION OVERWRITE fn::get_change_operations($atom_id: record<atom>) {
    return select out.atom_value.patch as operations 
    from triple 
    where 
      in = $atom_id 
      and predicate.id = fn::get_patch_predicate_id();
};

DEFINE FUNCTION OVERWRITE fn::interpreted_atom_value($atom_id: record<atom>) {
    
    let $values = select atom_value.* from atom where id = $atom_id;
    let $original = $values[0].atom_value;

    let $operations = fn::get_change_operations($atom_id);

    let $all_ops = array::reduce($operations, |$one, $two| $one.operations.concat($two.operations) );

    return value::patch($original, $all_ops);
};
