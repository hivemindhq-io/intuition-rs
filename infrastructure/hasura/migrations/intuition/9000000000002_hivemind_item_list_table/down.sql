DROP TRIGGER IF EXISTS item_list_insert_update_term_text_trigger ON item_list;
DROP INDEX IF EXISTS idx_atom_value_item_list_id;
ALTER TABLE atom_value DROP COLUMN IF EXISTS item_list_id;
DROP TABLE IF EXISTS item_list;
