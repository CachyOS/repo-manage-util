#include "packages_search_handler.hpp"
#include "http_utils.hpp"
#include "package_utils.hpp"
#include "pages_utils.hpp"
#include "queries.hpp"

#include <algorithm>  // for for_each
#include <optional>   // for optional
#include <ranges>     // for ranges::*
#include <string>     // for string
#include <utility>    // for move
#include <vector>     // for vector

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

#include <userver/server/handlers/exceptions.hpp>
#include <userver/storages/postgres/cluster.hpp>
#include <userver/storages/redis/client.hpp>
#include <userver/utils/from_string.hpp>
#include <userver/storages/postgres/io/json_types.hpp>
#include <userver/yaml_config/merge_schemas.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

#include <fmt/format.h>
#include <fmt/compile.h>

namespace {

using PackageResultRow = service::pg::utils::BriefPackagePageResultRow;

auto get_arg_helper(std::string_view arg) noexcept -> std::optional<std::int32_t> {
    if (arg.empty()) {
        return std::nullopt;
    }
    return userver::utils::FromString<std::int32_t>(arg);
}

constexpr auto round_pages(std::int64_t total_amount, std::int32_t page_size) noexcept {
    return utils::round_pages(static_cast<std::size_t>(total_amount), static_cast<std::size_t>(page_size));
}

constexpr auto make_split_view(std::string_view str, char delim) noexcept {
    constexpr auto functor = [](auto&& rng) {
        return std::string_view(&*rng.begin(), static_cast<size_t>(std::ranges::distance(rng)));
    };
    constexpr auto second = [](auto&& rng) { return rng != ""; };

    return str
        | std::ranges::views::split(delim)
        | std::ranges::views::transform(functor)
        | std::ranges::views::filter(second);
}

constexpr auto split_text(std::string_view text) noexcept -> std::optional<std::vector<std::string>> {
    std::vector<std::string> split_vec{};
    std::ranges::for_each(make_split_view(text, ','), [&](auto&& rng) { split_vec.emplace_back(rng); });
    // make empty values, equivalent to NULL in SQL
    if (split_vec.empty()) {
        return std::nullopt;
    }
    return std::make_optional(std::move(split_vec));
}

constexpr auto get_cache_key(std::int32_t current_page, std::int32_t page_size, std::string_view search_query, std::string_view repof, std::string_view archf) noexcept -> std::string {
    return fmt::format(FMT_COMPILE("{}-{}-{}-{}-{}"), current_page, page_size, search_query, repof, archf);
}

auto serialize_json(const userver::formats::json::Value& json_val) -> std::string {
    std::string buffer{};
    userver::storages::postgres::io::detail::JsonValueToBuffer(json_val, buffer);
    return buffer;
}

}  // namespace

namespace service::pg {

userver::formats::json::Value PackagesSearchHandler::HandleRequestJsonThrow(
    const userver::server::http::HttpRequest& request,
    const userver::formats::json::Value&,
    userver::server::request::RequestContext&) const {
    static constexpr auto kInvalidPage     = "Invalid page!";
    static constexpr auto kInvalidPageSize = "Invalid page size!";

    // set http headers
    http::utils::set_response_http_headers(request);

    // get paging args
    const auto& current_page = get_arg_helper(request.GetArg("current_page")).value_or(1);
    const auto& page_size    = get_arg_helper(request.GetArg("page_size")).value_or(100);
    // get search arg
    const auto& search_query = request.GetArg("search");

    // validate
    if (current_page <= 0) {
        throw userver::server::handlers::ClientError(
            userver::server::handlers::ExternalBody{kInvalidPage});
    }
    if (page_size <= 0) {
        throw userver::server::handlers::ClientError(
            userver::server::handlers::ExternalBody{kInvalidPageSize});
    }

    // check redis cache
    const auto& cache_key = get_cache_key(current_page, page_size, search_query, request.GetArg("repo"), request.GetArg("arch"));
    const auto& cache_val = redis_client_->Get(cache_key, redis_cc_).Get();
    if (cache_val) {
        return userver::formats::json::FromString(*cache_val);
    }

    // get filter args
    const auto& repof = split_text(request.GetArg("repo"));
    const auto& archf = split_text(request.GetArg("arch"));
    // calculate needed offset
    const auto& offset = (current_page - 1) * page_size;

    auto js_obj   = userver::formats::json::ValueBuilder(userver::formats::json::Type::kObject);
    auto js_array = userver::formats::json::ValueBuilder(userver::formats::json::Type::kArray);

    js_obj["packages"]       = js_array;
    js_obj["total_pages"]    = 0;
    js_obj["total_packages"] = 0;

    using userver::storages::postgres::ClusterHostType;
    auto result = pg_cluster_->Execute(ClusterHostType::kSlave, query::kSelectPackagesByString, page_size, offset, search_query, repof, archf);
    if (result.IsEmpty()) {
        // nothing found
        return js_obj.ExtractValue();
    }
    const auto& packages_res = result.AsSingleRow<PackageResultRow>(userver::storages::postgres::kRowTag);
    const auto& total_pages  = round_pages(packages_res.total_packages, page_size);

    js_obj["total_pages"]    = total_pages;
    js_obj["total_packages"] = packages_res.total_packages;
    std::ranges::for_each(
        packages_res.packages,
        [&js_obj](const auto& row) { js_obj["packages"].PushBack(row); });

    const auto& res_val = js_obj.ExtractValue();

    const auto& json_string = serialize_json(res_val);
    redis_client_->Set(cache_key, json_string, cache_ttl_, redis_cc_).Wait();
    return res_val;
}

userver::yaml_config::Schema PackagesSearchHandler::GetStaticConfigSchema() {
    return userver::yaml_config::MergeSchemas<userver::server::handlers::HttpHandlerJsonBase>(R"(
    # yaml
    type: object
    description: |
      Packages handler
    additionalProperties: false
    properties:
      redisdb:
          type: string
          description: (*required*) redis database name
      cache-ttl:
          type: string
          description: (*required*) utils::StringToDuration suitable duration string to store cache
  )");
}

}  // namespace service::pg
