#include "mirrors_mirrorlist_parser.hpp"

#include <algorithm>
#include <array>
#include <cctype>
#include <cstdint>
#include <optional>
#include <ranges>
#include <string>
#include <string_view>
#include <unordered_set>

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

#include <userver/http/url.hpp>
#include <userver/utils/from_string.hpp>
#include <userver/utils/text_light.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

using namespace std::string_view_literals;

namespace {

namespace text = userver::utils::text;

enum class PendingServerState : std::uint8_t {
    kNone,
    kEnabled,
    kDisabled,
};

struct PendingMirrorMetadata {
    std::string country_code;
    std::int32_t tier{2};
};

struct ParsedServerDirective {
    bool matched{false};
    std::optional<std::string_view> url;
};

auto trim_and_validate_url(std::string_view raw) -> std::optional<std::string_view> {
    auto candidate = text::TrimView(raw);
    if (!candidate.empty() && candidate.front() == '=') {
        candidate.remove_prefix(1);
        candidate = text::TrimView(candidate);
    }
    const auto scheme = userver::http::ExtractSchemeView(candidate);
    if (scheme != "http"sv && scheme != "https"sv) {
        return std::nullopt;
    }
    return candidate;
}

auto normalize_mirror_base_url(std::string_view raw) -> std::optional<std::string> {
    const auto candidate = trim_and_validate_url(raw);
    if (!candidate) {
        return std::nullopt;
    }

    std::string normalized{*candidate};

    static constexpr std::array kPacmanSuffixes{
        "/repo/$arch/$repo"sv,
        "/$repo/$arch"sv,
        "/$arch/$repo"sv,
        "/$repo"sv,
        "/$arch"sv,
    };
    if (const auto suffix = std::ranges::find_if(
            kPacmanSuffixes, [&](std::string_view s) { return text::EndsWith(normalized, s); });
        suffix != std::ranges::end(kPacmanSuffixes)) {
        normalized.erase(normalized.size() - suffix->size());
    }

    normalized.erase(normalized.find_last_not_of('/') + 1);
    normalized.push_back('/');
    return normalized;
}

auto parse_value_after_key(std::string_view raw, std::string_view key) -> std::optional<std::string_view> {
    const auto key_pos = raw.find(key);
    if (key_pos == std::string_view::npos) {
        return std::nullopt;
    }

    auto value = text::TrimView(raw.substr(key_pos + key.size()));
    if (value.empty()) {
        return std::nullopt;
    }

    // Keep only the first whitespace-delimited token.
    value = value.substr(0, value.find_first_of(" \t\n\v\f\r"));

    // Drop trailing punctuation.
    auto reversed   = value | std::ranges::views::reverse;
    const auto keep = std::ranges::find_if(
        reversed, [](unsigned char c) { return std::ispunct(c) == 0 || c == '-'; });
    value.remove_suffix(static_cast<std::size_t>(std::ranges::distance(reversed.begin(), keep)));

    if (value.empty()) {
        return std::nullopt;
    }
    return value;
}

void update_metadata_from_comment(std::string_view comment, PendingMirrorMetadata& metadata) {
    if (const auto tier_value = parse_value_after_key(comment, "tier="sv); tier_value.has_value()) {
        if (const auto parsed_tier = userver::utils::FromStringNoThrow<std::int64_t>(text::TrimView(*tier_value));
            parsed_tier.has_value() && (parsed_tier.value() == 1 || parsed_tier.value() == 2)) {
            metadata.tier = static_cast<int>(parsed_tier.value());
        }
    }

    if (const auto country_code_value = parse_value_after_key(comment, "code="sv);
        country_code_value.has_value()) {
        metadata.country_code = *country_code_value
            | std::ranges::views::transform([](unsigned char c) { return static_cast<char>(std::toupper(c)); })
            | std::ranges::to<std::string>();
    }
}

auto try_extract_url(std::string_view raw) -> std::optional<std::string_view> {
    return trim_and_validate_url(raw);
}

auto parse_server_directive(std::string_view raw) -> ParsedServerDirective {
    static constexpr auto kServerDirective = "Server"sv;

    auto candidate = text::TrimView(raw);
    if (!text::StartsWith(candidate, kServerDirective)) {
        return {};
    }

    candidate.remove_prefix(kServerDirective.size());
    candidate = text::TrimView(candidate);
    if (candidate.empty()) {
        return {.matched = true, .url = std::nullopt};
    }

    if (candidate.front() == '=') {
        candidate.remove_prefix(1);
        candidate = text::TrimView(candidate);
    }

    if (candidate.empty()) {
        return {.matched = true, .url = std::nullopt};
    }
    return {.matched = true, .url = candidate};
}

}  // namespace

namespace service::mirrors {

std::vector<MirrorMetadata> ParseMirrorlist(std::string_view body) {
    std::vector<MirrorMetadata> mirrors;

    std::unordered_set<std::string> seen_urls;
    PendingMirrorMetadata current_metadata;
    auto pending_server_state = PendingServerState::kNone;

    const auto reset_pending_entry = [&]() {
        current_metadata     = PendingMirrorMetadata{};
        pending_server_state = PendingServerState::kNone;
    };

    const auto append_mirror = [&](std::string_view raw_url) {
        const auto normalized = normalize_mirror_base_url(raw_url);
        if (!normalized) {
            return;
        }
        if (seen_urls.insert(*normalized).second) {
            mirrors.push_back(MirrorMetadata{
                .country_code = current_metadata.country_code,
                .url          = *normalized,
                .tier         = current_metadata.tier,
            });
        }
        reset_pending_entry();
    };

    for (const auto raw_line : text::SplitIntoStringViewVector(body, "\n")) {
        auto line = text::TrimView(raw_line);
        if (line.empty()) {
            continue;
        }

        const auto commented_out = line.front() == '#';
        auto content             = line;
        if (commented_out) {
            content.remove_prefix(std::min(content.find_first_not_of('#'), content.size()));
            content = text::TrimView(content);
        }

        if (pending_server_state != PendingServerState::kNone) {
            if (const auto pending_url = try_extract_url(content); pending_url.has_value()) {
                if (pending_server_state == PendingServerState::kEnabled && !commented_out) {
                    append_mirror(*pending_url);
                } else {
                    reset_pending_entry();
                }
                continue;
            }
            reset_pending_entry();
        }

        if (const auto server = parse_server_directive(content); server.matched) {
            if (server.url.has_value()) {
                if (!commented_out) {
                    append_mirror(*server.url);
                } else {
                    reset_pending_entry();
                }
            } else {
                pending_server_state = commented_out
                    ? PendingServerState::kDisabled
                    : PendingServerState::kEnabled;
            }
            continue;
        }

        if (commented_out) {
            update_metadata_from_comment(content, current_metadata);
            continue;
        }

        if (const auto standalone_url = try_extract_url(content); standalone_url.has_value()) {
            append_mirror(*standalone_url);
        }
    }

    return mirrors;
}

}  // namespace service::mirrors
