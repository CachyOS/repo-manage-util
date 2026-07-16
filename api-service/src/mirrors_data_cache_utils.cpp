#include "mirrors_data_cache_utils.hpp"

#include <chrono>
#include <optional>
#include <string>

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

#include <userver/formats/json/value_builder.hpp>
#include <userver/formats/serialize/common_containers.hpp>
#include <userver/utils/datetime_light.hpp>
#include <userver/utils/trivial_map.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace {

using service::mirrors::MirrorStatus;
using service::mirrors::RepoStatus;

auto iso_or_null(const std::optional<std::chrono::system_clock::time_point>& timestamp)
    -> std::optional<std::string> {
    if (!timestamp.has_value()) {
        return std::nullopt;
    }
    return userver::utils::datetime::UtcTimestring(*timestamp, userver::utils::datetime::kIsoFormat);
}

constexpr userver::utils::TrivialBiMap kRepoStatusNames = [](auto selector) {
    return selector()
        .Case("synced", RepoStatus::kSynced)
        .Case("out-of-sync", RepoStatus::kOutOfSync)
        .Case("error", RepoStatus::kError);
};

constexpr userver::utils::TrivialBiMap kMirrorStatusNames = [](auto selector) {
    return selector()
        .Case("healthy", MirrorStatus::kHealthy)
        .Case("partial", MirrorStatus::kPartial)
        .Case("out-of-sync", MirrorStatus::kOutOfSync)
        .Case("error", MirrorStatus::kError);
};

}  // namespace

namespace service::mirrors {

auto to_json_string(RepoStatus status) -> std::string_view {
    return kRepoStatusNames.TryFindBySecond(status).value();
}

auto to_json_string(MirrorStatus status) -> std::string_view {
    return kMirrorStatusNames.TryFindBySecond(status).value();
}

userver::formats::json::Value Serialize(
    const RepoCheck& check,
    userver::formats::serialize::To<userver::formats::json::Value>) {
    auto builder                = userver::formats::json::ValueBuilder(userver::formats::json::Type::kObject);
    builder["path"]             = check.path;
    builder["status"]           = to_json_string(check.status);
    builder["sync_lag_seconds"] = check.sync_lag_seconds;
    builder["last_updated"]     = iso_or_null(check.last_updated);
    return builder.ExtractValue();
}

userver::formats::json::Value Serialize(
    const MirrorEntry& entry,
    userver::formats::serialize::To<userver::formats::json::Value>) {
    auto builder            = userver::formats::json::ValueBuilder(userver::formats::json::Type::kObject);
    builder["country_code"] = entry.country_code;
    builder["url"]          = entry.url;
    builder["out_of_date"]  = entry.out_of_date;
    builder["tier"]         = entry.tier;

    builder["last_sync"]           = iso_or_null(entry.last_sync);
    builder["overall_status"]      = to_json_string(entry.overall_status);
    builder["average_lag_seconds"] = entry.average_lag_seconds;
    builder["delay_seconds"]       = entry.delay_seconds;
    builder["checks"]              = entry.checks;
    return builder.ExtractValue();
}

}  // namespace service::mirrors
