#pragma once

#include <string_view>

namespace service::http::headers {

inline constexpr std::string_view kOrigin                   = "Origin";
inline constexpr std::string_view kAccessControlAllowOrigin = "Access-Control-Allow-Origin";

}  // namespace service::http::headers
