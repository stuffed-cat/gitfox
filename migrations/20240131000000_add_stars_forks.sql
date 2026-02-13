-- Project stars table
CREATE TABLE project_stars (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, user_id)
);

CREATE INDEX idx_project_stars_project ON project_stars(project_id);
CREATE INDEX idx_project_stars_user ON project_stars(user_id);

-- Project forks table
CREATE TABLE project_forks (
    id BIGSERIAL PRIMARY KEY,
    source_project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    forked_project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    forked_by BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(source_project_id, forked_by)
);

CREATE INDEX idx_project_forks_source ON project_forks(source_project_id);
CREATE INDEX idx_project_forks_forked ON project_forks(forked_project_id);

-- Add forked_from_id column to projects table
ALTER TABLE projects ADD COLUMN forked_from_id BIGINT REFERENCES projects(id) ON DELETE SET NULL;

-- Add stars_count and forks_count as cached counters (optional, for performance)
ALTER TABLE projects ADD COLUMN stars_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN forks_count INTEGER NOT NULL DEFAULT 0;

-- Trigger to update stars_count
CREATE OR REPLACE FUNCTION update_project_stars_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE projects SET stars_count = stars_count + 1 WHERE id = NEW.project_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE projects SET stars_count = stars_count - 1 WHERE id = OLD.project_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_stars_count
AFTER INSERT OR DELETE ON project_stars
FOR EACH ROW EXECUTE FUNCTION update_project_stars_count();

-- Trigger to update forks_count
CREATE OR REPLACE FUNCTION update_project_forks_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE projects SET forks_count = forks_count + 1 WHERE id = NEW.source_project_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE projects SET forks_count = forks_count - 1 WHERE id = OLD.source_project_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_forks_count
AFTER INSERT OR DELETE ON project_forks
FOR EACH ROW EXECUTE FUNCTION update_project_forks_count();
