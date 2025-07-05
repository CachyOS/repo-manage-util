#include "http_utils.hpp"
#include "http_headers.hpp"

#include <string>
#include <string_view>

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

#include <userver/http/common_headers.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace service::http::utils {

void set_response_http_headers(const userver::server::http::HttpRequest& request) noexcept {
    using namespace std::string_view_literals;

    auto& response = request.GetHttpResponse();
    if (request.HasHeader(http::headers::kOrigin)) {
        const auto& req_origin = request.GetHeader(http::headers::kOrigin);
        std::string_view allowed_origin{};
        if (req_origin.starts_with("http://localhost:3000"sv)) {
            allowed_origin = "http://localhost:3000"sv;
        }
        response.SetHeader(http::headers::kAccessControlAllowOrigin, std::string(allowed_origin));
    }
}

}  // namespace service::http::utils
