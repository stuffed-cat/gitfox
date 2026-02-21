-- 修复超时触发器时机：应该在 timeout 后延迟触发，作为兜底机制

-- 1. 删除旧的触发器
DROP TRIGGER IF EXISTS trigger_auto_fail_timeout_jobs ON jobs;
DROP TRIGGER IF EXISTS trigger_auto_fail_timeout_jobs_insert ON jobs;

-- 2. 修改触发器函数：只在超时 1 小时后才介入（作为兜底）
CREATE OR REPLACE FUNCTION auto_fail_timeout_jobs()
RETURNS TRIGGER AS $$
DECLARE
    v_running_seconds BIGINT;
BEGIN
    -- 只处理 running 状态的 job
    IF NEW.status != 'running' THEN
        RETURN NEW;
    END IF;

    -- 检查是否超时 + 已经过了 1 小时兜底时间
    -- 正常情况下 Redis 或 devops 实例应该在 timeout_at 时就处理了
    -- 这里等 1 小时是给足够的缓冲时间，避免误杀
    IF NEW.timeout_at IS NOT NULL AND NEW.timeout_at + INTERVAL '1 hour' <= NOW() THEN
        -- 计算运行时长
        IF NEW.started_at IS NOT NULL THEN
            v_running_seconds := EXTRACT(EPOCH FROM (NOW() - NEW.started_at));
        ELSE
            v_running_seconds := 0;
        END IF;

        -- 直接修改状态为 failed
        NEW.status := 'failed';
        NEW.finished_at := NOW();
        NEW.updated_at := NOW();
        NEW.error_message := 'Job exceeded maximum execution time limit (failsafe: database trigger kicked in after ' || v_running_seconds || 's, timeout+1h grace period expired)';
        NEW.watcher_instance := NULL;

        -- 发送 NOTIFY（低优先级，因为这是兜底机制）
        PERFORM pg_notify(
            'job_timeout',
            json_build_object(
                'job_id', NEW.id,
                'pipeline_id', NEW.pipeline_id,
                'watcher_instance', OLD.watcher_instance,
                'timeout_at', NEW.timeout_at,
                'running_seconds', v_running_seconds,
                'trigger_type', 'failsafe'
            )::text
        );
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 3. 重新创建触发器
CREATE TRIGGER trigger_auto_fail_timeout_jobs
    BEFORE UPDATE ON jobs
    FOR EACH ROW
    WHEN (NEW.status = 'running' AND NEW.timeout_at IS NOT NULL)
    EXECUTE FUNCTION auto_fail_timeout_jobs();

-- 4. 删除 check_dead_watcher_jobs 函数（改用 Redis 检查）
DROP FUNCTION IF EXISTS check_dead_watcher_jobs();

-- 5. 删除 devops_instances 表（改用 Redis）
DROP TABLE IF EXISTS devops_instances;

COMMENT ON FUNCTION auto_fail_timeout_jobs IS 'Failsafe: Auto-fail jobs that exceeded timeout + 1 hour grace period (last resort after Redis/devops failed to handle)';
