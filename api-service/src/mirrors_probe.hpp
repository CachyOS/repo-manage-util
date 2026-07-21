#pragma once

#include <chrono>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wshorten-64-to-32"
#pragma clang diagnostic ignored "-Wsign-conversion"
#pragma clang diagnostic ignored "-Wnon-virtual-dtor"
#elif defined(__GNUC__)
#pragma GCC diagnostic push
#endif

#include <userver/clients/http/client.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace service::mirrors {

using Timestamp = std::chrono::system_clock::time_point;

struct RepoProbe {
    userver::clients::http::Client& client;
    std::string_view base_url;
    std::chrono::milliseconds timeout;
};

auto fetch_repo_timestamps(const RepoProbe& probe, const std::vector<std::string>& repo_paths) noexcept
    -> std::vector<std::optional<Timestamp>>;

}  // namespace service::mirrors
