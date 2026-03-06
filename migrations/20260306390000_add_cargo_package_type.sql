-- 扩展 package_type 枚举添加 cargo 类型
-- 必须在单独的事务中执行，才能在后续迁移中使用新值
ALTER TYPE package_type ADD VALUE IF NOT EXISTS 'cargo';
