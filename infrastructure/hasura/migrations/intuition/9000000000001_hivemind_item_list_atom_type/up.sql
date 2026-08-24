-- Hive Mind fork: recognize schema.org ItemList as a first-class atom type.
-- Out-of-band version prefix (9000000000001) keeps fork migrations sorted after
-- any future upstream timestamp so rebases can never collide.
ALTER TYPE atom_type ADD VALUE IF NOT EXISTS 'ItemList';
