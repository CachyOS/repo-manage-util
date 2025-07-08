CREATE SCHEMA IF NOT EXISTS helper_schema;

CREATE EXTENSION IF NOT EXISTS pg_trgm;

DO $$
BEGIN
IF NOT EXISTS (select 1 from pg_type where typname = 'package_metadata' AND typnamespace = (SELECT oid FROM pg_namespace WHERE nspname = 'helper_schema')) then
    CREATE TYPE helper_schema.package_metadata AS (
        pkg_base      TEXT,
        pkg_desc      TEXT,
        pkg_groups    TEXT[],
        pkg_url       TEXT,
        pkg_license   TEXT[],
        pkg_arch      TEXT,
        pkg_builddate TIMESTAMPTZ,
        pkg_packager  TEXT,
        pkg_csize     BIGINT,
        pkg_isize     BIGINT,
        pkg_sha256sum TEXT,
        pkg_pgpsig    TEXT
    );
END IF;
END $$;

DO $$
BEGIN
IF NOT EXISTS (select 1 from pg_type where typname = 'package_dependencies' AND typnamespace = (SELECT oid FROM pg_namespace WHERE nspname = 'helper_schema')) then
    CREATE TYPE helper_schema.package_dependencies AS (
        pkg_replaces   TEXT[],
        pkg_depends    TEXT[],
        pkg_optdepends TEXT[],
        pkg_makedepends TEXT[],
        pkg_checkdepends TEXT[],
        pkg_conflicts  TEXT[],
        pkg_provides   TEXT[],
        pkg_files      TEXT[]
    );
END IF;
END $$;

DO $$
BEGIN
IF NOT EXISTS (select 1 from pg_type where typname = 'repository_info' AND typnamespace = (SELECT oid FROM pg_namespace WHERE nspname = 'helper_schema')) then
    CREATE TYPE helper_schema.repository_info AS (
        repo_desc TEXT
    );
END IF;
END $$;

CREATE TABLE IF NOT EXISTS repositories (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_name  TEXT UNIQUE NOT NULL,
    repo_desc  TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS packages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_name       TEXT NOT NULL REFERENCES repositories(repo_name) ON DELETE CASCADE ON UPDATE CASCADE,
    pkg_name        TEXT NOT NULL,
    pkg_version     TEXT NOT NULL,
    pkg_filename    TEXT NOT NULL,
    pkg_base        TEXT,
    pkg_desc        TEXT,
    pkg_groups      TEXT[],
    pkg_url         TEXT,
    pkg_license     TEXT[],
    pkg_arch        TEXT,
    pkg_builddate   TIMESTAMPTZ,
    pkg_packager    TEXT,
    pkg_csize       BIGINT,
    pkg_isize       BIGINT,
    pkg_sha256sum   TEXT,
    pkg_pgpsig      TEXT,
    pkg_replaces    TEXT[],
    pkg_depends     TEXT[],
    pkg_optdepends  TEXT[],
    pkg_makedepends TEXT[],
    pkg_checkdepends TEXT[],
    pkg_conflicts   TEXT[],
    pkg_provides    TEXT[],
    pkg_files       TEXT[],
    updated         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- The natural key for a package instance.
    UNIQUE(repo_name, pkg_name)
);

CREATE INDEX IF NOT EXISTS idx_packages_name ON packages(pkg_name);
CREATE INDEX IF NOT EXISTS idx_packages_filename ON packages(pkg_filename);
CREATE INDEX IF NOT EXISTS idx_packages_arch ON packages(pkg_arch);

CREATE INDEX IF NOT EXISTS idx_packages_search ON packages USING gin (pkg_name gin_trgm_ops, pkg_desc gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_packages_depends ON packages USING gin (pkg_depends);
CREATE INDEX IF NOT EXISTS idx_packages_provides ON packages USING gin (pkg_provides);
CREATE INDEX IF NOT EXISTS idx_packages_files ON packages USING gin (pkg_files);

CREATE OR REPLACE FUNCTION helper_schema.insert_or_update_package(
    _repo_name text,
    _pkg_name text,
    _pkg_version text,
    _pkg_filename text,
    _metadata helper_schema.package_metadata,
    _dependencies helper_schema.package_dependencies
)
    RETURNS UUID
    AS $$
DECLARE
    v_package_id UUID;
BEGIN
    INSERT INTO packages (
        repo_name, pkg_name, pkg_version, pkg_filename,
        pkg_base, pkg_desc, pkg_groups, pkg_url, pkg_license, pkg_arch, pkg_builddate, pkg_packager,
        pkg_csize, pkg_isize, pkg_sha256sum, pkg_pgpsig,
        pkg_replaces, pkg_depends, pkg_optdepends, pkg_makedepends, pkg_checkdepends,
        pkg_conflicts, pkg_provides, pkg_files
    ) VALUES (
        _repo_name, _pkg_name, _pkg_version, _pkg_filename,
        (_metadata).pkg_base, (_metadata).pkg_desc, (_metadata).pkg_groups, (_metadata).pkg_url,
        (_metadata).pkg_license, (_metadata).pkg_arch, (_metadata).pkg_builddate, (_metadata).pkg_packager,
        (_metadata).pkg_csize, (_metadata).pkg_isize, (_metadata).pkg_sha256sum, (_metadata).pkg_pgpsig,
        (_dependencies).pkg_replaces, (_dependencies).pkg_depends, (_dependencies).pkg_optdepends,
        (_dependencies).pkg_makedepends, (_dependencies).pkg_checkdepends, (_dependencies).pkg_conflicts,
        (_dependencies).pkg_provides, (_dependencies).pkg_files
    )
    ON CONFLICT (repo_name, pkg_name) DO UPDATE SET
        pkg_version      = EXCLUDED.pkg_version,
        pkg_filename     = EXCLUDED.pkg_filename,
        pkg_base         = EXCLUDED.pkg_base,
        pkg_desc         = EXCLUDED.pkg_desc,
        pkg_groups       = EXCLUDED.pkg_groups,
        pkg_url          = EXCLUDED.pkg_url,
        pkg_license      = EXCLUDED.pkg_license,
        pkg_arch         = EXCLUDED.pkg_arch,
        pkg_builddate    = EXCLUDED.pkg_builddate,
        pkg_packager     = EXCLUDED.pkg_packager,
        pkg_csize        = EXCLUDED.pkg_csize,
        pkg_isize        = EXCLUDED.pkg_isize,
        pkg_sha256sum    = EXCLUDED.pkg_sha256sum,
        pkg_pgpsig       = EXCLUDED.pkg_pgpsig,
        pkg_replaces     = EXCLUDED.pkg_replaces,
        pkg_depends      = EXCLUDED.pkg_depends,
        pkg_optdepends   = EXCLUDED.pkg_optdepends,
        pkg_makedepends  = EXCLUDED.pkg_makedepends,
        pkg_checkdepends = EXCLUDED.pkg_checkdepends,
        pkg_conflicts    = EXCLUDED.pkg_conflicts,
        pkg_provides     = EXCLUDED.pkg_provides,
        pkg_files        = EXCLUDED.pkg_files,
        updated          = NOW()
    RETURNING id INTO v_package_id;

    RETURN v_package_id;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION helper_schema.remove_package(_repo_name text, _pkg_name text)
    RETURNS BOOLEAN
    AS $$
DECLARE
    v_deleted_count INTEGER;
BEGIN
    DELETE FROM packages
    WHERE
        repo_name = _repo_name AND
        pkg_name = _pkg_name;

    GET DIAGNOSTICS v_deleted_count = ROW_COUNT;
    RETURN v_deleted_count > 0;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION helper_schema.get_package_info(_repo_name text, _pkg_name text)
    RETURNS SETOF packages
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM packages p
    WHERE
        p.repo_name = _repo_name AND
        p.pkg_name = _pkg_name
    ORDER BY p.pkg_version DESC;
END;
$$
LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION helper_schema.get_repo_packages(_repo_name text)
    RETURNS SETOF packages
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM packages p
    WHERE p.repo_name = _repo_name
    ORDER BY p.pkg_name, p.pkg_version DESC;
END;
$$
LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION helper_schema.search_packages(_repo_name text, _search_pattern text)
    RETURNS SETOF packages
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM packages p
    WHERE
        p.repo_name = _repo_name AND
        (p.pkg_name ILIKE '%' || _search_pattern || '%' OR
        p.pkg_desc ILIKE '%' || _search_pattern || '%')
    ORDER BY p.pkg_name, p.pkg_version DESC;
END;
$$
LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION helper_schema.get_packages_by_arch(_repo_name text, _arch text)
    RETURNS SETOF packages
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM packages p
    WHERE
        p.repo_name = _repo_name AND
        p.pkg_arch = _arch
    ORDER BY p.pkg_name, p.pkg_version DESC;
END;
$$
LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION helper_schema.insert_or_update_repository(_repo_name text, _info helper_schema.repository_info)
    RETURNS UUID
    AS $$
DECLARE
    v_repo_id UUID;
BEGIN
    INSERT INTO repositories (repo_name, repo_desc)
    VALUES (_repo_name, (_info).repo_desc)
    ON CONFLICT (repo_name)
    DO UPDATE SET
        repo_desc = EXCLUDED.repo_desc,
        updated_at = NOW()
    RETURNING id INTO v_repo_id;

    RETURN v_repo_id;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION helper_schema.remove_repository(_repo_name text)
    RETURNS VOID
    AS $$
BEGIN
    DELETE FROM repositories
    WHERE
        repo_name = _repo_name;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION helper_schema.get_all_repositories()
    RETURNS SETOF repositories
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM repositories r
    ORDER BY r.repo_name;
END;
$$
LANGUAGE plpgsql STABLE;

CREATE OR REPLACE VIEW package_summary AS
SELECT
    repo_name,
    pkg_name,
    pkg_version,
    pkg_arch,
    pkg_csize,
    pkg_desc,
    updated
FROM packages;

CREATE OR REPLACE VIEW repo_summary AS
SELECT
    p.repo_name,
    count(*) AS total_packages,
    count(DISTINCT p.pkg_name) AS unique_packages,
    min(p.updated) AS oldest_package_update,
    max(p.updated) AS newest_package_update,
    r.repo_desc
FROM packages p
JOIN repositories r ON p.repo_name = r.repo_name
GROUP BY p.repo_name, r.repo_desc
ORDER BY p.repo_name;

CREATE OR REPLACE FUNCTION helper_schema.get_repo_stats(_repo_name text)
    RETURNS SETOF repo_summary
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM repo_summary
    WHERE repo_summary.repo_name = _repo_name;
END;
$$
LANGUAGE plpgsql STABLE;
