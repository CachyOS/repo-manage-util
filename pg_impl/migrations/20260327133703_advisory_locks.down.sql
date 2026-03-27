-- Recreate the redundant unique index that was dropped.
CREATE UNIQUE INDEX IF NOT EXISTS uniq_pkg_idx ON packages (repo_name, pkg_name);

-- Revert insert_or_update_package to version without advisory lock.
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

-- Revert remove_package to version without advisory lock.
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
