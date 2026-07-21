#pragma once

#include <chrono>
#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wshorten-64-to-32"
#pragma clang diagnostic ignored "-Wsign-conversion"
#pragma clang diagnostic ignored "-Wdouble-promotion"
#pragma clang diagnostic ignored "-Wimplicit-int-float-conversion"
#pragma clang diagnostic ignored "-Wimplicit-int-conversion"
#pragma clang diagnostic ignored "-Wshadow"
#pragma clang diagnostic ignored "-Wnon-virtual-dtor"
#elif defined(__GNUC__)
#pragma GCC diagnostic push
#endif

#include <userver/formats/json_fwd.hpp>
#include <userver/formats/serialize/to.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace service::mirrors {

enum class RepoStatus : std::uint8_t {
    kSynced,
    kOutOfSync,
    kError,
};

enum class MirrorStatus : std::uint8_t {
    kHealthy,
    kPartial,
    kOutOfSync,
    kError,
};

struct RepoCheck {
    std::string path;
    std::optional<std::chrono::system_clock::time_point> last_updated;
    std::optional<std::int64_t> sync_lag_seconds;
    RepoStatus status = RepoStatus::kError;
};

struct MirrorEntry {
    bool out_of_date;
    std::int32_t tier;
    std::string country_code;
    std::string url;
    std::optional<std::chrono::system_clock::time_point> last_sync;
    std::vector<RepoCheck> checks;
    std::optional<std::int64_t> average_lag_seconds;
    std::optional<std::int64_t> delay_seconds;
    MirrorStatus overall_status = MirrorStatus::kError;
};

struct MirrorsData {
    std::vector<MirrorEntry> mirrors;
};

userver::formats::json::Value Serialize(
    const RepoCheck& check,
    userver::formats::serialize::To<userver::formats::json::Value>);

userver::formats::json::Value Serialize(
    const MirrorEntry& entry,
    userver::formats::serialize::To<userver::formats::json::Value>);

}  // namespace service::mirrors
