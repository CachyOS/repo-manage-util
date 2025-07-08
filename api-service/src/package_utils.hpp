#pragma once

#include <cstddef>  // For int64_t
#include <ctime>    // for time_t

#include <optional>  // for optional
#include <string>    // for string
#include <vector>    // for vector

#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wshorten-64-to-32"
#pragma clang diagnostic ignored "-Wsign-conversion"
#pragma clang diagnostic ignored "-Wdouble-promotion"
#pragma clang diagnostic ignored "-Wimplicit-int-float-conversion"
#pragma clang diagnostic ignored "-Wimplicit-int-conversion"
#pragma clang diagnostic ignored "-Wshadow"
#pragma clang diagnostic ignored "-Wnon-virtual-dtor"
#pragma clang diagnostic ignored "-Wold-style-cast"
#elif defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wnull-dereference"
#pragma GCC diagnostic ignored "-Wuseless-cast"
#pragma GCC diagnostic ignored "-Wold-style-cast"
#endif

#include <userver/formats/json_fwd.hpp>
#include <userver/storages/postgres/io/io_fwd.hpp>
#include <userver/storages/postgres/io/pg_types.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace service::pg::utils {

struct BriefPackageRow {
    std::string pkg_name;
    std::string repo_name;
    std::string pkg_arch;
    std::string pkg_version;
    std::string pkg_desc;
    std::time_t pkg_builddate;
};

struct BriefPackagePageResultRow {
    std::int64_t total_packages;
    std::vector<BriefPackageRow> packages;
};

struct PublicPackageRow {
    std::string repo_name;
    std::string pkg_name;
    std::string pkg_version;
    std::optional<std::string> pkg_base;
    std::optional<std::string> pkg_desc;
    std::optional<std::vector<std::string>> pkg_groups;
    std::optional<std::string> pkg_url;
    std::optional<std::vector<std::string>> pkg_license;
    std::optional<std::string> pkg_arch;
    std::optional<std::int64_t> pkg_builddate;
    std::optional<std::string> pkg_packager;
    std::optional<std::int64_t> pkg_csize;
    std::optional<std::int64_t> pkg_isize;
    std::optional<std::string> pkg_sha256sum;
    std::optional<std::string> pkg_pgpsig;
    std::optional<std::vector<std::string>> pkg_replaces;
    std::optional<std::vector<std::string>> pkg_depends;
    std::optional<std::vector<std::string>> pkg_optdepends;
    std::optional<std::vector<std::string>> pkg_makedepends;
    std::optional<std::vector<std::string>> pkg_checkdepends;
    std::optional<std::vector<std::string>> pkg_conflicts;
    std::optional<std::vector<std::string>> pkg_provides;
    std::optional<std::vector<std::string>> pkg_files;
    std::int64_t updated;
};

userver::formats::json::Value Serialize(
    const BriefPackageRow& row,
    userver::formats::serialize::To<userver::formats::json::Value>);

userver::formats::json::Value Serialize(
    const PublicPackageRow& row,
    userver::formats::serialize::To<userver::formats::json::Value>);

}  // namespace service::pg::utils

namespace userver::storages::postgres::io {
// This specialization MUST go to the header together with the mapped type
template <>
struct CppToUserPg<service::pg::utils::BriefPackageRow> {
    static constexpr DBTypeName postgres_name = "helper_schema.brief_package";
};
template <>
struct CppToUserPg<service::pg::utils::BriefPackagePageResultRow> {
    static constexpr DBTypeName postgres_name = "helper_schema.brief_package_page_result";
};
template <>
struct CppToUserPg<service::pg::utils::PublicPackageRow> {
    static constexpr DBTypeName postgres_name = "helper_schema.public_package";
};
}  // namespace userver::storages::postgres::io
