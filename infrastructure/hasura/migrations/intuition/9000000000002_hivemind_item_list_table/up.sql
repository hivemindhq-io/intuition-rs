-- Hive Mind fork: typed table for schema.org ItemList atoms.
-- Columns mirror exactly what the Explorer's buildItemListJson mints: id, name,
-- description. Deliberately no itemListElement / numberOfItems -- embedded
-- membership contradicts the lens model, where membership lives in stakeable
-- "this item belongs on this list" claims.

CREATE TABLE IF NOT EXISTS item_list (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT,
  description TEXT
);

ALTER TABLE atom_value ADD COLUMN IF NOT EXISTS item_list_id TEXT;
CREATE INDEX IF NOT EXISTS idx_atom_value_item_list_id ON atom_value(item_list_id);

-- Mirrors book_insert_update_term_text_trigger; update_term_text_function() is
-- generic over NEW.name / NEW.description so it needs no modification.
DROP TRIGGER IF EXISTS item_list_insert_update_term_text_trigger ON item_list;
CREATE TRIGGER item_list_insert_update_term_text_trigger
AFTER INSERT OR UPDATE ON item_list
FOR EACH ROW
EXECUTE FUNCTION update_term_text_function();

COMMENT ON COLUMN atom_value.item_list_id IS 'Reference to item_list table if atom value is an ItemList.';
