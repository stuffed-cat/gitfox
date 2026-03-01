-- Fix subgroup paths to include full path (parent/subgroup)
-- This migration updates existing subgroups to store their complete path
-- instead of just the relative path
-- NOTE: Only processes subgroups where path doesn't contain '/' (pure relative paths)

-- First, extract the relative path for all subgroups
-- (strip parent path prefix if it exists)
WITH RECURSIVE parent_info AS (
    SELECT 
        n.id,
        n.path as current_path,
        n.parent_id,
        pn.path as parent_path
    FROM namespaces n
    LEFT JOIN namespaces pn ON n.parent_id = pn.id
    WHERE n.parent_id IS NOT NULL
),
relative_paths AS (
    SELECT 
        id,
        current_path,
        parent_id,
        parent_path,
        -- Extract relative path: if current_path starts with parent_path/, remove it
        CASE 
            WHEN current_path LIKE parent_path || '/%' 
            THEN substring(current_path from length(parent_path) + 2)
            ELSE current_path
        END as relative_path
    FROM parent_info
),
-- Now rebuild full paths using recursive CTE
namespace_full_paths AS (
    -- Base case: top-level namespaces (no parent)
    SELECT 
        id,
        path::text as full_path
    FROM namespaces
    WHERE parent_id IS NULL
    
    UNION ALL
    
    -- Recursive case: child namespaces using their relative path
    SELECT 
        n.id,
        (nfp.full_path || '/' || rp.relative_path)::text as full_path
    FROM namespaces n
    INNER JOIN namespace_full_paths nfp ON n.parent_id = nfp.id
    INNER JOIN relative_paths rp ON n.id = rp.id
)
-- Update namespaces table with correct full paths
UPDATE namespaces n
SET path = nfp.full_path
FROM namespace_full_paths nfp
WHERE n.id = nfp.id 
  AND n.path != nfp.full_path;

-- Do the same for groups table
WITH RECURSIVE parent_info AS (
    SELECT 
        g.id,
        g.path as current_path,
        g.parent_id,
        pg.path as parent_path
    FROM groups g
    LEFT JOIN groups pg ON g.parent_id = pg.id
    WHERE g.parent_id IS NOT NULL
),
relative_paths AS (
    SELECT 
        id,
        current_path,
        parent_id,
        parent_path,
        CASE 
            WHEN current_path LIKE parent_path || '/%' 
            THEN substring(current_path from length(parent_path) + 2)
            ELSE current_path
        END as relative_path
    FROM parent_info
),
group_full_paths AS (
    -- Base case: top-level groups
    SELECT 
        id,
        path::text as full_path
    FROM groups
    WHERE parent_id IS NULL
    
    UNION ALL
    
    -- Recursive case: subgroups
    SELECT 
        g.id,
        (gfp.full_path || '/' || rp.relative_path)::text as full_path
    FROM groups g
    INNER JOIN group_full_paths gfp ON g.parent_id = gfp.id
    INNER JOIN relative_paths rp ON g.id = rp.id
)
UPDATE groups g
SET path = gfp.full_path
FROM group_full_paths gfp
WHERE g.id = gfp.id 
  AND g.path != gfp.full_path;
