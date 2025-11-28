cmake_minimum_required(VERSION 3.15)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

##
## PROJECT
## name and version
##
project(api-postgresql-service
        VERSION 0.0.1
        LANGUAGES CXX)

##
## INCLUDE
##
include(GNUInstallDirs)
include(StandardProjectSettings)
include(CompilerWarnings)
include(EnableCcache)
include(Linker)
include(StaticAnalyzers)
include(Sanitizers)
include(CPM)

find_package(PkgConfig REQUIRED)
find_package(Threads REQUIRED)
# NOTE: hack to make userver find correct openssl dependencies
find_package(OpenSSL REQUIRED)

# import as toplevel postgresql as its needed by libpqxx
add_subdirectory(third_party/postgresql)

add_compile_options(-I${CMAKE_SOURCE_DIR}/third_party/postgresql/src/include -I${CMAKE_SOURCE_DIR}/third_party/postgresql -I${CMAKE_SOURCE_DIR}/third_party/postgresql/src/interfaces/libpq)
include_directories(${CMAKE_SOURCE_DIR}/third_party/postgresql/src/include ${CMAKE_SOURCE_DIR}/third_party/postgresql ${CMAKE_SOURCE_DIR}/third_party/postgresql/src/interfaces/libpq)

pkg_check_modules(
  LIBALPM
  REQUIRED
  IMPORTED_TARGET
  libalpm>=13.0.0)

CPMAddPackage(
  NAME fmt
  GITHUB_REPOSITORY fmtlib/fmt
  GIT_TAG 11.2.0
  EXCLUDE_FROM_ALL YES
)
CPMAddPackage(
  NAME spdlog
  GITHUB_REPOSITORY gabime/spdlog
  GIT_TAG b18a234ed6af38638678a3338c0d7ed90210ae6c
  EXCLUDE_FROM_ALL YES
)
CPMAddPackage(
  NAME libpqxx
  GITHUB_REPOSITORY jtv/libpqxx
  GIT_TAG 9dd71102e1f3e10b0f14eed253873ee6ce2a4880
  EXCLUDE_FROM_ALL YES
)
CPMAddPackage(
  NAME Corrosion
  GITHUB_REPOSITORY corrosion-rs/corrosion
  GIT_TAG v0.5.2
  EXCLUDE_FROM_ALL YES
)

# include userver things
list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/third_party/userver/cmake")
list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/third_party/userver/cmake/modules")
list(APPEND CMAKE_MODULE_PATH "${CMAKE_BINARY_DIR}/cmake_generated")

##
## CONFIGURATION
##
if(CMAKE_CXX_COMPILER_ID MATCHES ".*Clang")
   set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} -flto=thin -fwhole-program-vtables -fsplit-lto-unit")
   set(CMAKE_C_FLAGS_RELEASE "${CMAKE_C_FLAGS_RELEASE} -flto=thin -fsplit-lto-unit")
   set(CMAKE_EXE_LINKER_FLAGS_RELEASE "${CMAKE_EXE_LINKER_FLAGS_RELEASE} -flto=thin -fwhole-program-vtables")
elseif(CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
   set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} -flto -fwhole-program -fuse-linker-plugin")
endif()

if(ENABLE_TESTING)
  enable_testing()
endif()

set(USERVER_PYTHON "python3")
set(USERVER_IMPL_SETUP_ENV_WAS_RUN_FOR_THIS_DIR ON)

add_subdirectory(third_party/userver)

set(USERVER_TESTSUITE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/third_party/userver/testsuite")
set(POSTGRESQL_SERVICE_CONFIG_DIR "\"${PROJECT_SOURCE_DIR}/\"")

# embed SQL file
userver_add_sql_library(
    ${PROJECT_NAME}_sql
    NAMESPACE
    service
    OUTPUT_DIR
    ${CMAKE_CURRENT_BINARY_DIR}
    SQL_FILES
    postgresql/schemas/*.sql
)

# Link this 'library' to set the c++ standard / compile-time options requested
add_library(project_options INTERFACE)
target_compile_features(project_options INTERFACE cxx_std_23)

##
## Target
##
add_executable(api-postgresql-service
   src/http_utils.cpp
   src/package_utils.cpp
   src/package_handler.cpp
   src/package_files_handler.cpp
   src/split_package_handler.cpp
   src/packages_search_handler.cpp
   src/packages_suggest_handler.cpp
   src/diff_checker.cpp
   src/arch_repo_checker.cpp
   src/main.cpp
   )

add_executable(arch-diff
   src/diff_checker.cpp
   src/arch_repo_checker.cpp
   src/arch-diff.cpp
   )

add_library(${PROJECT_NAME}_rustutils
   src/diff_component_rust.hpp src/diff_component_rust.cpp
   )

# Link this 'library' to use the warnings specified in CompilerWarnings.cmake
add_library(project_warnings INTERFACE)
set_project_warnings(project_warnings)

# Add linker configuration
configure_linker(project_options)

# sanitizer options if supported by compiler
enable_sanitizers(project_options)

corrosion_import_crate(MANIFEST_PATH "backend-rustlib/Cargo.toml" FLAGS "${CARGO_FLAGS}")
corrosion_add_cxxbridge(backend-rustlib-cxxbridge CRATE backend_rustlib FILES lib.rs)

target_compile_definitions(${PROJECT_NAME} PRIVATE POSTGRESQL_SERVICE_CONFIG_DIR=${POSTGRESQL_SERVICE_CONFIG_DIR})
target_compile_definitions(${PROJECT_NAME} PRIVATE NDEBUG)
target_include_directories(${PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_BINARY_DIR})
target_include_directories(backend-rustlib-cxxbridge PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/backend-rustlib/src)
target_link_libraries(${PROJECT_NAME} PRIVATE project_warnings project_options spdlog::spdlog fmt::fmt userver::core userver::postgresql userver::redis backend-rustlib-cxxbridge ${PROJECT_NAME}_rustutils ${PROJECT_NAME}_sql libpqxx::pqxx pq::pq)
target_link_libraries(arch-diff PRIVATE project_warnings project_options fmt::fmt userver::core userver::postgresql backend-rustlib-cxxbridge ${PROJECT_NAME}_rustutils ${PROJECT_NAME}_sql libpqxx::pqxx pq::pq)
target_link_libraries(${PROJECT_NAME}_rustutils PRIVATE project_warnings project_options userver::core userver::postgresql backend-rustlib-cxxbridge PkgConfig::LIBALPM)

if(ENABLE_TESTING)
userver_testsuite_add_simple(
    PYTHON_BINARY "${TESTSUITE_PYTHON_BINARY}"
    CONFIG_PATH "${CMAKE_CURRENT_SOURCE_DIR}/postgres_service.yaml"
)
endif()

option(ENABLE_UNITY "Enable Unity builds of projects" OFF)
if(ENABLE_UNITY)
   # Add for any project you want to apply unity builds for
   set_target_properties(${PROJECT_NAME} PROPERTIES UNITY_BUILD ON)
endif()
