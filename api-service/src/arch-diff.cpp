// Copyright (C) 2025 Vladislav Nepogodin
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along
// with this program; if not, write to the Free Software Foundation, Inc.,
// 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
#include "diff_checker.hpp"

#include <csignal>

#include <chrono>
#include <iostream>
#include <string>
#include <string_view>

#include <boost/program_options.hpp>

#ifdef __clang__
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wshorten-64-to-32"
#pragma clang diagnostic ignored "-Wsign-conversion"
#pragma clang diagnostic ignored "-Wdouble-promotion"
#pragma clang diagnostic ignored "-Wimplicit-int-float-conversion"
#pragma clang diagnostic ignored "-Wimplicit-int-conversion"
#pragma clang diagnostic ignored "-Wshadow"
#pragma clang diagnostic ignored "-Wnon-virtual-dtor"
#elifdef __GNUC__
#pragma GCC diagnostic push
// #pragma GCC diagnostic ignored "-Wold-style-cast"
#endif

#include <userver/engine/async.hpp>
#include <userver/engine/run_standalone.hpp>
#include <userver/engine/task/current_task.hpp>
#include <userver/logging/log.hpp>
#include <userver/logging/logger.hpp>
#include <userver/utils/periodic_task.hpp>

#ifdef __clang__
#pragma clang diagnostic pop
#elifdef __GNUC__
#pragma GCC diagnostic pop
#endif

namespace {

struct Config {
    std::string log_level = "error";
    size_t worker_threads = 1;
};

Config ParseConfig(int argc, const char* const argv[]) {
    namespace po = boost::program_options;

    Config c;
    po::options_description desc("Allowed options");

    // clang-format off
    desc.add_options()
      ("help,h", "produce help message")
      ("log-level", po::value(&c.log_level)->default_value(c.log_level), "log level (trace, debug, info, warning, error)")
      ("worker-threads,t", po::value(&c.worker_threads)->default_value(c.worker_threads), "worker thread count")
    ;
    // clang-format on

    po::variables_map vm;
    try {
        po::store(po::command_line_parser(argc, argv).options(desc).run(), vm);
        po::notify(vm);
    } catch (const std::exception& ex) {
        fmt::println(stderr, "Cannot parse command line: {}", ex.what());
        std::exit(1);  // NOLINT(concurrency-mt-unsafe)
    }

    if (vm.contains("help")) {
        std::cout << desc << '\n';
        std::exit(0);  // NOLINT(concurrency-mt-unsafe)
    }

    return c;
}

}  // namespace

static void run_main() {
    auto diff_comp = service::alpm::DiffComponent(nullptr);
    if (!diff_comp.refresh_handles()) {
        LOG_ERROR() << "Failed to init ALPM handles";
        return;
    }

    diff_comp.run_check();
    if (!diff_comp.update_local_copy()) {
        LOG_ERROR() << "Failed to update local db copy";
    }
}

int main(int argc, const char* const argv[]) {
    const auto config = ParseConfig(argc, argv);

    signal(SIGPIPE, SIG_IGN);

    const userver::logging::DefaultLoggerGuard guard{
        userver::logging::MakeStderrLogger("default", userver::logging::Format::kJson, userver::logging::LevelFromString(config.log_level))};

    userver::engine::RunStandalone(config.worker_threads, [&] {
        userver::engine::AsyncNoTracing(userver::engine::current_task::GetBlockingTaskProcessor(), &run_main).Get();
    });
}
