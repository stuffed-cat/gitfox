-- 修复 package_files.file_sha256 列长度
-- npm 使用 SHA-512 (128 字符十六进制)，原来的 VARCHAR(64) 只能存 SHA-256

-- 重命名列以反映实际用途（存储 SHA-512）
ALTER TABLE package_files 
    ALTER COLUMN file_sha256 TYPE VARCHAR(128);

-- 添加注释说明
COMMENT ON COLUMN package_files.file_sha256 IS 'SHA-256 或 SHA-512 哈希值（十六进制）';
