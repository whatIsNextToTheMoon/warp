DROP INDEX IF EXISTS blocks_pane_leaf_uuid_id_idx;
DROP INDEX IF EXISTS blocks_pane_leaf_uuid_block_id_idx;

ALTER TABLE blocks DROP COLUMN stylized_output_bytes;
