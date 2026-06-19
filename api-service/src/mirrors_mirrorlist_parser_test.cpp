#include "mirrors_mirrorlist_parser.hpp"

#include <string_view>

#include <gtest/gtest.h>

namespace service::mirrors {
namespace {

TEST(MirrorlistParser, EmptyInputYieldsNoMirrors) {
    EXPECT_TRUE(ParseMirrorlist("").empty());
    EXPECT_TRUE(ParseMirrorlist("\n\n   \n").empty());
}

TEST(MirrorlistParser, EnabledServerDirectiveIsParsed) {
    const auto mirrors = ParseMirrorlist("Server = https://mirror.example.org/cachyos/$arch/$repo");
    ASSERT_EQ(mirrors.size(), 1u);
    EXPECT_EQ(mirrors[0].url, "https://mirror.example.org/cachyos/");
    EXPECT_EQ(mirrors[0].tier, 2);
    EXPECT_TRUE(mirrors[0].country_code.empty());
}

TEST(MirrorlistParser, CommentedOutServerIsSkipped) {
    const auto mirrors = ParseMirrorlist("#Server = https://disabled.example.org/$repo/$arch");
    EXPECT_TRUE(mirrors.empty());
}

TEST(MirrorlistParser, TierAndCountryCommentsAnnotateFollowingServer) {
    const auto mirrors = ParseMirrorlist(
        "## tier=1 code=fr\n"
        "Server = https://fr.example.org/cachyos/$arch/$repo");
    ASSERT_EQ(mirrors.size(), 1u);
    EXPECT_EQ(mirrors[0].tier, 1);
    EXPECT_EQ(mirrors[0].country_code, "FR");
}

TEST(MirrorlistParser, InvalidTierIsIgnoredAndDefaultsToTwo) {
    const auto mirrors = ParseMirrorlist(
        "# tier=9 code=de\n"
        "Server = https://de.example.org/$arch/$repo");
    ASSERT_EQ(mirrors.size(), 1u);
    EXPECT_EQ(mirrors[0].tier, 2);
    EXPECT_EQ(mirrors[0].country_code, "DE");
}

TEST(MirrorlistParser, BareServerLineFollowedByEnabledUrl) {
    const auto mirrors = ParseMirrorlist(
        "Server =\n"
        "https://standalone.example.org/$repo");
    ASSERT_EQ(mirrors.size(), 1u);
    EXPECT_EQ(mirrors[0].url, "https://standalone.example.org/");
}

TEST(MirrorlistParser, BareServerLineFollowedByCommentedUrlIsSkipped) {
    const auto mirrors = ParseMirrorlist(
        "Server =\n"
        "#https://commented.example.org/$repo");
    EXPECT_TRUE(mirrors.empty());
}

TEST(MirrorlistParser, StandaloneUrlIsParsed) {
    const auto mirrors = ParseMirrorlist("https://plain.example.org/repo/$arch/$repo");
    ASSERT_EQ(mirrors.size(), 1u);
    EXPECT_EQ(mirrors[0].url, "https://plain.example.org/");
}

TEST(MirrorlistParser, NonHttpLinesAreIgnored) {
    const auto mirrors = ParseMirrorlist(
        "ftp://nope.example.org/$repo\n"
        "just some text\n"
        "Server = ftp://also-nope.example.org/$repo");
    EXPECT_TRUE(mirrors.empty());
}

TEST(MirrorlistParser, DuplicateNormalizedUrlsAreDeduplicated) {
    const auto mirrors = ParseMirrorlist(
        "Server = https://dup.example.org/cachyos/$arch/$repo\n"
        "Server = https://dup.example.org/cachyos/$repo/$arch\n"
        "Server = https://dup.example.org/cachyos/");
    ASSERT_EQ(mirrors.size(), 1u);
    EXPECT_EQ(mirrors[0].url, "https://dup.example.org/cachyos/");
}

TEST(MirrorlistParser, TrailingSlashIsAlwaysEnforced) {
    const auto mirrors = ParseMirrorlist("Server = https://noslash.example.org/cachyos");
    ASSERT_EQ(mirrors.size(), 1u);
    EXPECT_EQ(mirrors[0].url, "https://noslash.example.org/cachyos/");
}

TEST(MirrorlistParser, MetadataResetsBetweenEntries) {
    const auto mirrors = ParseMirrorlist(
        "# tier=1 code=fr\n"
        "Server = https://fr.example.org/$arch/$repo\n"
        "Server = https://other.example.org/$arch/$repo");
    ASSERT_EQ(mirrors.size(), 2u);
    EXPECT_EQ(mirrors[0].country_code, "FR");
    EXPECT_EQ(mirrors[0].tier, 1);
    EXPECT_TRUE(mirrors[1].country_code.empty());
    EXPECT_EQ(mirrors[1].tier, 2);
}

}  // namespace
}  // namespace service::mirrors
