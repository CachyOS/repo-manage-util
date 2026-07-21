#include "mirrors_probe.hpp"

#include <cstdint>

#include <chrono>
#include <optional>
#include <ranges>
#include <string>
#include <string_view>
#include <vector>

#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wshorten-64-to-32"
#pragma clang diagnostic ignored "-Wsign-conversion"
#pragma clang diagnostic ignored "-Wold-style-cast"
#elif defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wold-style-cast"
#endif

#include <userver/clients/http/response.hpp>
#include <userver/http/url.hpp>
#include <userver/logging/log.hpp>
#include <userver/utils/from_string.hpp>
#include <userver/utils/text_light.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace {

using namespace std::string_view_literals;

// re-exp
using Timestamp = service::mirrors::Timestamp;
using RepoProbe = service::mirrors::RepoProbe;

inline constexpr auto kLastUpdateEndpoint = "lastupdate"sv;

auto join_timestamp_url(std::string_view base_url, std::string_view repo_path) noexcept -> std::string {
    std::string url{base_url};
    if (url.empty() || url.back() != '/') {
        url.push_back('/');
    }

    for (const auto segment : userver::utils::text::SplitIntoStringViewVector(repo_path, "/")) {
        url.append(userver::http::UrlEncodePathSegment(segment));
        url.push_back('/');
    }
    url.append(kLastUpdateEndpoint);
    return url;
}

auto fetch_repo_timestamp(const RepoProbe& probe, std::string_view repo_path) noexcept -> std::optional<Timestamp> {
    try {
        const auto& response = probe.client.CreateRequest()
                                   .get(join_timestamp_url(probe.base_url, repo_path))
                                   .timeout(probe.timeout)
                                   .perform();
        if (!response->IsOk()) {
            return std::nullopt;
        }

        const auto& parsed = userver::utils::FromStringNoThrow<std::int64_t>(
            userver::utils::text::TrimView(response->body_view()));
        if (!parsed.has_value()) {
            return std::nullopt;
        }
        return Timestamp{std::chrono::microseconds{parsed.value()}};
    } catch (const std::exception& ex) {
        LOG_DEBUG("Failed to fetch repo timestamp for '{}' from '{}': {}", repo_path, probe.base_url, ex.what());
    }
    return std::nullopt;
}

}  // namespace

namespace service::mirrors {

auto fetch_repo_timestamps(
    const RepoProbe& probe, const std::vector<std::string>& repo_paths) noexcept -> std::vector<std::optional<Timestamp>> {
    return repo_paths
        | std::ranges::views::transform([&](const auto& repo_path) { return fetch_repo_timestamp(probe, repo_path); })
        | std::ranges::to<std::vector>();
}

}  // namespace service::mirrors
