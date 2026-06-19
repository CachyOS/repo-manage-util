# Set a default build type if none was specified
if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
  message(STATUS "Setting build type to 'RelWithDebInfo' as none was specified.")
  set(CMAKE_BUILD_TYPE
      RelWithDebInfo
      CACHE STRING "Choose the type of build." FORCE)
  # Set the possible values of build type for cmake-gui, ccmake
  set_property(
    CACHE CMAKE_BUILD_TYPE
    PROPERTY STRINGS
             "Debug"
             "Release"
             "MinSizeRel"
             "RelWithDebInfo")
endif()

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_POLICY_VERSION_MINIMUM 3.5)

set(BUILD_SHARED_LIBS OFF CACHE INTERNAL "" FORCE)
set(SPDLOG_FMT_EXTERNAL ON CACHE INTERNAL "" FORCE)
set(USERVER_FEATURE_MONGODB OFF CACHE INTERNAL "" FORCE)
set(USERVER_FEATURE_REDIS ON CACHE INTERNAL "" FORCE)
set(USERVER_FEATURE_GRPC OFF CACHE INTERNAL "" FORCE)
set(USERVER_FEATURE_CLICKHOUSE OFF CACHE INTERNAL "" FORCE)
set(USERVER_FEATURE_RABBITMQ OFF CACHE INTERNAL "" FORCE)
set(USERVER_FEATURE_POSTGRESQL ON CACHE INTERNAL "" FORCE)
set(USERVER_FEATURE_MYSQL OFF CACHE INTERNAL "" FORCE)
set(USERVER_FEATURE_STACKTRACE OFF CACHE INTERNAL "" FORCE)
set(USERVER_FEATURE_UTEST OFF CACHE INTERNAL "" FORCE)
set(USERVER_FEATURE_GRPC_CHANNELZ OFF CACHE INTERNAL "" FORCE)
set(USERVER_FEATURE_JEMALLOC OFF CACHE INTERNAL "" FORCE)
set(USERVER_FEATURE_UNIVERSAL OFF CACHE INTERNAL "" FORCE)
set(USERVER_IS_THE_ROOT_PROJECT OFF CACHE INTERNAL "" FORCE)
set(USERVER_DOWNLOAD_PACKAGE_CCTZ OFF CACHE INTERNAL "" FORCE)
set(USERVER_CHECK_PACKAGE_VERSIONS OFF CACHE INTERNAL "" FORCE)
set(USERVER_FEATURE_PATCH_LIBPQ ON CACHE INTERNAL "" FORCE)
set(USERVER_PG_SERVER_INCLUDE_DIR "${CMAKE_SOURCE_DIR}/third_party/postgresql/src/include" CACHE INTERNAL "" FORCE)
set(PostgreSQL_INCLUDE_DIR "${CMAKE_SOURCE_DIR}/third_party/postgresql/src/interfaces/libpq" CACHE INTERNAL "" FORCE)
set(PostgreSQL_LIBRARY "${CMAKE_SOURCE_DIR}/third_party/postgresql/src/interfaces/libpq/libpq.a" CACHE INTERNAL "" FORCE)
add_definitions(-DSPDLOG_FMT_EXTERNAL -DSPDLOG_COMPILED_LIB)

# Generate compile_commands.json to make it easier to work with clang based tools
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

if(CMAKE_CXX_COMPILER_ID MATCHES ".*Clang")
  #add_compile_options(-nostdlib++ -stdlib=libc++ -nodefaultlibs -fexperimental-library)
  #add_link_options(-stdlib=libc++)

  add_compile_options(-fstrict-vtable-pointers)

  if(CMAKE_CXX_COMPILER_VERSION VERSION_LESS 16)
    # Set new experimental pass manager, it's a performance, build time and binary size win.
    # Can be removed after https://reviews.llvm.org/D66490 merged and released to at least two versions of clang.
    add_compile_options(-fexperimental-new-pass-manager)
  endif()
endif()

option(ENABLE_TESTING "Enable tests and testsuite" ON)
option(ENABLE_IPO "Enable Interprocedural Optimization, aka Link Time Optimization (LTO)" OFF)

if(ENABLE_IPO)
  include(CheckIPOSupported)
  check_ipo_supported(
    RESULT
    result
    OUTPUT
    output)
  if(result)
    set(CMAKE_INTERPROCEDURAL_OPTIMIZATION ON)
  else()
    message(SEND_ERROR "IPO is not supported: ${output}")
  endif()
endif()
if(CMAKE_CXX_COMPILER_ID MATCHES ".*Clang")
  add_compile_options(-fcolor-diagnostics)
elseif(CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
  add_compile_options(-fdiagnostics-color=always)
else()
  message(STATUS "No colored compiler diagnostic set for '${CMAKE_CXX_COMPILER_ID}' compiler.")
endif()

# Enables STL container checker if not building a release.
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
  add_definitions(-D_GLIBCXX_ASSERTIONS)
  add_definitions(-D_LIBCPP_ENABLE_THREAD_SAFETY_ANNOTATIONS=1)
  add_definitions(-D_LIBCPP_ENABLE_ASSERTIONS=1)
endif()

# Enables tests
if(ENABLE_TESTING)
  set(USERVER_FEATURE_TESTSUITE ON CACHE INTERNAL "" FORCE)
  set(USERVER_FEATURE_UTEST ON CACHE INTERNAL "" FORCE)
endif()
