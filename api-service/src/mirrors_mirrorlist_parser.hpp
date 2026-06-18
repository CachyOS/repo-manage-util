#pragma once

#include <string>
#include <string_view>
#include <vector>

namespace service::mirrors {

struct MirrorMetadata {
    std::string country_code;
    std::string url;
    int tier{2};
};

std::vector<MirrorMetadata> ParseMirrorlist(std::string_view body);

}  // namespace service::mirrors
