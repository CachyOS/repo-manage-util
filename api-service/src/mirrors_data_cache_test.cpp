#include "mirrors_probe.hpp"

#include <chrono>
#include <memory>
#include <optional>
#include <string>
#include <utility>
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

#include <userver/clients/http/client.hpp>
#include <userver/clients/http/config.hpp>
#include <userver/clients/http/standalone_client.hpp>
#include <userver/utest/http_server_mock.hpp>
#include <userver/utest/utest.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace {

using namespace std::chrono_literals;

inline constexpr std::int64_t kLastUpdateMicros = 1782249621332331;

auto repo_paths() -> const std::vector<std::string>& {
    static const std::vector<std::string> kPaths{
        "x86_64/cachyos",
        "x86_64_v3/cachyos-v3",
        "x86_64_v4/cachyos-v4",
    };
    return kPaths;
}

auto mockclient() -> std::shared_ptr<userver::clients::http::Client> {
    userver::clients::http::ClientSettings settings{};
    settings.io_threads = 1;
    return userver::clients::http::CreateStandaloneHttpClient(
        std::move(settings), {}, userver::engine::current_task::GetTaskProcessor());
}

}  // namespace

namespace service::mirrors {

UTEST(MirrorsFetchTimestamps, SequentialProbesReuseOneConnection) {
    userver::utest::HttpServerMock mock{[](const userver::utest::HttpServerMock::HttpRequest&) {
        return userver::utest::HttpServerMock::HttpResponse{.headers = {}, .body = std::to_string(kLastUpdateMicros)};
    }};

    const auto client     = mockclient();
    const auto timestamps = fetch_repo_timestamps({.client = *client, .base_url = mock.GetBaseUrl(), .timeout = 5s}, repo_paths());

    ASSERT_EQ(timestamps.size(), repo_paths().size());
    const Timestamp expected{std::chrono::microseconds{kLastUpdateMicros}};
    for (const auto& timestamp : timestamps) {
        ASSERT_TRUE(timestamp.has_value());
        EXPECT_EQ(*timestamp, expected);
    }
    EXPECT_EQ(mock.GetConnectionsOpenedCount(), 1);
}

UTEST(MirrorsFetchTimestamps, MapsFailuresToNullopt) {
    userver::utest::HttpServerMock mock{[](const userver::utest::HttpServerMock::HttpRequest& request) {
        if (request.path.contains("cachyos-v3")) {
            return userver::utest::HttpServerMock::HttpResponse{.response_status = 404, .headers = {}, .body = "not found"};
        }
        if (request.path.contains("cachyos-v4")) {
            return userver::utest::HttpServerMock::HttpResponse{.headers = {}, .body = "not-a-number"};
        }
        return userver::utest::HttpServerMock::HttpResponse{.headers = {}, .body = std::to_string(kLastUpdateMicros)};
    }};

    const auto client     = mockclient();
    const auto timestamps = fetch_repo_timestamps({.client = *client, .base_url = mock.GetBaseUrl(), .timeout = 5s}, repo_paths());

    ASSERT_EQ(timestamps.size(), 3u);
    EXPECT_TRUE(timestamps[0].has_value());
    EXPECT_FALSE(timestamps[1].has_value());
    EXPECT_FALSE(timestamps[2].has_value());
}

}  // namespace service::mirrors
