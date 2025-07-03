#pragma once

#include <cstddef>  // for size_t

namespace utils {

// @brief Computes the number of pages needed to hold total_amount items,
// given that each page can hold page_size items.
// This is equivalent to ceil(total_amount / page_size).
constexpr std::size_t round_pages(std::size_t total_amount, std::size_t page_size) {
    return (total_amount + page_size - 1) / page_size;
}

static_assert(round_pages(2107, 2200) == 1);
static_assert(round_pages(2107, 2107) == 1);
static_assert(round_pages(2107, 1100) == 2);
static_assert(round_pages(0, 100) == 0);
static_assert(round_pages(1, 100) == 1);
static_assert(round_pages(100, 100) == 1);
static_assert(round_pages(101, 100) == 2);
static_assert(round_pages(200, 100) == 2);
static_assert(round_pages(0, 1) == 0);
static_assert(round_pages(1, 1) == 1);
static_assert(round_pages(5, 1) == 5);

}  // namespace utils
