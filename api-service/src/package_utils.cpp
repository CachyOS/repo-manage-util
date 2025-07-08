#include "package_utils.hpp"

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

#include <userver/formats/json.hpp>
#include <userver/formats/serialize/common_containers.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace service::pg::utils {

userver::formats::json::Value Serialize(
    const BriefPackageRow& row,
    userver::formats::serialize::To<userver::formats::json::Value>) {
    auto pkg_object = userver::formats::json::ValueBuilder(userver::formats::json::Type::kObject);

    pkg_object["pkg_name"]      = row.pkg_name;
    pkg_object["repo_name"]     = row.repo_name;
    pkg_object["pkg_arch"]      = row.pkg_arch;
    pkg_object["pkg_version"]   = row.pkg_version;
    pkg_object["pkg_desc"]      = row.pkg_desc;
    pkg_object["pkg_builddate"] = row.pkg_builddate;

    return pkg_object.ExtractValue();
}

userver::formats::json::Value Serialize(
    const PublicPackageRow& row,
    userver::formats::serialize::To<userver::formats::json::Value>) {
    auto pkg_object = userver::formats::json::ValueBuilder(userver::formats::json::Type::kObject);

    pkg_object["repo_name"]        = row.repo_name;
    pkg_object["pkg_name"]         = row.pkg_name;
    pkg_object["pkg_version"]      = row.pkg_version;
    pkg_object["pkg_base"]         = row.pkg_base;
    pkg_object["pkg_desc"]         = row.pkg_desc;
    pkg_object["pkg_groups"]       = row.pkg_groups;
    pkg_object["pkg_url"]          = row.pkg_url;
    pkg_object["pkg_license"]      = row.pkg_license;
    pkg_object["pkg_arch"]         = row.pkg_arch;
    pkg_object["pkg_builddate"]    = row.pkg_builddate;
    pkg_object["pkg_packager"]     = row.pkg_packager;
    pkg_object["pkg_csize"]        = row.pkg_csize;
    pkg_object["pkg_isize"]        = row.pkg_isize;
    pkg_object["pkg_sha256sum"]    = row.pkg_sha256sum;
    pkg_object["pkg_pgpsig"]       = row.pkg_pgpsig;
    pkg_object["pkg_replaces"]     = row.pkg_replaces;
    pkg_object["pkg_depends"]      = row.pkg_depends;
    pkg_object["pkg_optdepends"]   = row.pkg_optdepends;
    pkg_object["pkg_makedepends"]  = row.pkg_makedepends;
    pkg_object["pkg_checkdepends"] = row.pkg_checkdepends;
    pkg_object["pkg_conflicts"]    = row.pkg_conflicts;
    pkg_object["pkg_provides"]     = row.pkg_provides;
    pkg_object["pkg_files"]        = row.pkg_files;
    pkg_object["updated"]          = row.updated;

    return pkg_object.ExtractValue();
}

}  // namespace service::pg::utils
