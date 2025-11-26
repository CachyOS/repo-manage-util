#pragma once

#include <userver/storages/postgres/query.hpp>

namespace service::pg::query {

inline const userver::storages::postgres::Query kSelectPackage{R"~(
SELECT * FROM helper_schema.get_package($1, $2, $3);
)~",
    userver::storages::postgres::Query::Name{"select-package-query"}};

inline const userver::storages::postgres::Query kSelectPackageFiles{R"~(
SELECT * FROM helper_schema.get_package_files($1, $2, $3);
)~",
    userver::storages::postgres::Query::Name{"select-package-files-query"}};

inline const userver::storages::postgres::Query kSelectSplitPackage{R"~(
SELECT * FROM helper_schema.get_split_package($1, $2);
)~",
    userver::storages::postgres::Query::Name{"select-split-package-query"}};

inline const userver::storages::postgres::Query kSelectPackagesByString{R"~(
SELECT * FROM helper_schema.get_page_search_packages_with_offset($1, $2, $3, $4, $5);
)~",
    userver::storages::postgres::Query::Name{"select-packages-by-string-query"}};

inline const userver::storages::postgres::Query kSelectTopPackageNames{R"~(
SELECT * FROM helper_schema.get_top_pkg_names($1, $2);
)~",
    userver::storages::postgres::Query::Name{"select-top-package-names-query"}};

inline const userver::storages::postgres::Query kInsertRepository{R"~(
SELECT helper_schema.insert_or_update_repository($1, NULL);
)~",
    userver::storages::postgres::Query::Name{"insert-repo-query"}};

inline const userver::storages::postgres::Query kInsertPackage{R"~(
SELECT helper_schema.insert_or_update_package($1, $2, $3, $4, $5, $6);
)~",
    userver::storages::postgres::Query::Name{"insert-package-query"}};

inline const userver::storages::postgres::Query kRemovePackage{R"~(
SELECT helper_schema.remove_package($1, $2);
)~",
    userver::storages::postgres::Query::Name{"remove-package-query"}};

}  // namespace service::pg::query
