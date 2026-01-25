-- 移除projects表的default_branch列
-- 默认分支应该从Git仓库HEAD读取，不应该存在数据库里
-- 这是GitLab/GitHub的正确做法：默认分支是Git配置，不是数据库字段

ALTER TABLE projects DROP COLUMN IF EXISTS default_branch;
