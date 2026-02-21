-- PostgreSQL 触发器 + NOTIFY 机制实现 job 超时处理

-- 1. devops 实例心跳表
CREATE TABLE IF NOT EXISTS devops_instances (
    instance_id VARCHAR(255) PRIMARY KEY,
    last_heartbeat TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_devops_instances_heartbeat ON devops_instances(last_heartbeat);

COMMENT ON TABLE devops_instances IS 'Tracks active devops instances heartbeat';

-- 2. 触发器函数：检测 job 超时，直接修改状态为 failed 并发送 NOTIFY
CREATE OR REPLACE FUNCTION auto_fail_timeout_jobs()
RETURNS TRIGGER AS $$
DECLARE
    v_watcher_alive BOOLEAN := FALSE;
    v_running_seconds BIGINT;
BEGIN
    -- 只处理 running 状态的 job
    IF NEW.status != 'running' THEN
        RETURN NEW;
    END IF;

    -- 检查是否超时
    IF NEW.timeout_at IS NOT NULL AND NEW.timeout_at <= NOW() THEN
        -- 检查 watcher 是否还活着（2分钟内有心跳）
        IF NEW.watcher_instance IS NOT NULL THEN
            SELECT EXISTS(
                SELECT 1 FROM devops_instances
                WHERE instance_id = NEW.watcher_instance
                AND last_heartbeat > NOW() - INTERVAL '2 minutes'
            ) INTO v_watcher_alive;
        END IF;

        -- 如果没有 watcher 或 watcher 已死，直接标记为 failed
        IF NEW.watcher_instance IS NULL OR NOT v_watcher_alive THEN
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
            NEW.error_message := 'Job exceeded maximum execution time limit (auto-failed by database trigger after ' || v_running_seconds || 's)';
            NEW.watcher_instance := NULL;

            -- 发送 NOTIFY 通知所有 devops 实例（用于日志记录等）
            PERFORM pg_notify(
                'job_timeout',
                json_build_object(
                    'job_id', NEW.id,
                    'pipeline_id', NEW.pipeline_id,
                    'watcher_instance', OLD.watcher_instance,
                    'timeout_at', NEW.timeout_at,
                    'running_seconds', v_running_seconds
                )::text
            );
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 3. 创建触发器：在 UPDATE/INSERT jobs 时自动检查并处理超时
CREATE TRIGGER trigger_auto_fail_timeout_jobs
    BEFORE UPDATE ON jobs
    FOR EACH ROW
    WHEN (NEW.status = 'running' AND NEW.timeout_at IS NOT NULL)
    EXECUTE FUNCTION auto_fail_timeout_jobs();

-- 也处理 INSERT 的情况（如果直接插入已超时的 running job）
CREATE TRIGGER trigger_auto_fail_timeout_jobs_insert
    BEFORE INSERT ON jobs
    FOR EACH ROW
    WHEN (NEW.status = 'running' AND NEW.timeout_at IS NOT NULL)
    EXECUTE FUNCTION auto_fail_timeout_jobs();

-- 4. 触发器函数：清理死亡 watcher 的 jobs
CREATE OR REPLACE FUNCTION check_dead_watcher_jobs()
RETURNS void AS $$
BEGIN
    -- 对所有 running 且 watcher 已死的 job 发送通知
    PERFORM pg_notify(
        'job_timeout',
        json_build_object(
            'job_id', j.id,
            'pipeline_id', j.pipeline_id,
            'watcher_instance', j.watcher_instance,
            'timeout_at', j.timeout_at,
            'reason', 'dead_watcher'
        )::text
    )
    FROM jobs j
    WHERE j.status = 'running'
    AND j.timeout_at IS NOT NULL
    AND j.timeout_at <= NOW()
    AND (
        j.watcher_instance IS NULL
        OR NOT EXISTS (
            SELECT 1 FROM devops_instances di
            WHERE di.instance_id = j.watcher_instance
            AND di.last_heartbeat > NOW() - INTERVAL '2 minutes'
        )
    );
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION check_dead_watcher_jobs IS 'Manually trigger timeout check for jobs with dead watchers';

-- 5. 定期调用函数（devops 启动后手动调用，不用 pg_cron）
-- 留给应用层在心跳时调用 SELECT check_dead_watcher_jobs();
