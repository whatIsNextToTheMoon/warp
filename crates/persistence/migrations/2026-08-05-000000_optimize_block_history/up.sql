ALTER TABLE blocks ADD COLUMN stylized_output_bytes BIGINT NOT NULL DEFAULT 0;

UPDATE blocks
SET stylized_output_bytes = LENGTH(stylized_output);

-- Keep the newest copy before enforcing the key used by block upserts. Older databases may
-- contain duplicates because block updates used to be implemented as DELETE followed by INSERT.
DELETE FROM blocks
WHERE id NOT IN (
    SELECT MAX(id)
    FROM blocks
    GROUP BY pane_leaf_uuid, block_id
);

CREATE UNIQUE INDEX blocks_pane_leaf_uuid_block_id_idx
ON blocks (pane_leaf_uuid, block_id);

-- This is a covering index for history-budget scans and also accelerates whole-pane deletes.
CREATE INDEX blocks_pane_leaf_uuid_id_idx
ON blocks (pane_leaf_uuid, id, stylized_output_bytes);

DELETE FROM blocks
WHERE NOT EXISTS (
    SELECT 1
    FROM terminal_panes
    WHERE terminal_panes.uuid = blocks.pane_leaf_uuid
);

-- Preserve a large command-count window for small outputs while bounding restored terminal data.
WITH ranked_blocks AS (
    SELECT
        id,
        ROW_NUMBER() OVER (
            PARTITION BY pane_leaf_uuid
            ORDER BY id DESC
        ) AS block_rank,
        SUM(MIN(stylized_output_bytes, 2097152)) OVER (
            PARTITION BY pane_leaf_uuid
            ORDER BY id DESC
            ROWS UNBOUNDED PRECEDING
        ) AS cumulative_output_bytes
    FROM blocks
)
DELETE FROM blocks
WHERE id IN (
    SELECT id
    FROM ranked_blocks
    WHERE block_rank > 1000 OR cumulative_output_bytes > 16777216
);

-- Existing installations can contain multi-megabyte blocks. Keep their most recent output so
-- migration immediately establishes the same per-block invariant as new writes.
UPDATE blocks
SET
    stylized_output = SUBSTR(stylized_output, -2097152),
    stylized_output_bytes = MIN(stylized_output_bytes, 2097152)
WHERE stylized_output_bytes > 2097152;
