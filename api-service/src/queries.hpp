#pragma once

#include <userver/storages/postgres/query.hpp>

namespace service::pg::query {

inline const userver::storages::postgres::Query kSelectPackage{R"~(
SELECT * FROM helper_schema.get_package($1, $2, $3);
)~",
    userver::storages::postgres::Query::Name{"select-package-query"}};

inline const userver::storages::postgres::Query kSelectPackagesByString{R"~(
SELECT * FROM helper_schema.get_page_search_packages_with_offset($1, $2, $3, $4, $5);
)~",
    userver::storages::postgres::Query::Name{"select-packages-by-string-query"}};

}  // namespace service::pg::query
