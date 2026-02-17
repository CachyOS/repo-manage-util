# repo-manage-util API

## Quick Start

```sh
# Install dependencies:
sudo pacman -Syu --noconfirm --noprogressbar --needed \
    git python cmake ninja clang llvm lld boost boost-libs libbsd tcl libxml2 util-linux zlib \
    sudo crypto++ yaml-cpp c-ares libev python-jinja \
    libnghttp2 postgresql meson pkg-config rust hiredis cctz http-parser

# Setup libpq and userver:
./setup_libpq.py $PWD
./setup_userver.py $PWD

# (Optional) Install previous versions of zlib:
sudo pacman -Syu zlib lib32-zlib

# Build with tests:
./configure.sh -t=Debug --use_clang --enable_tests && ./build.sh

# (Optional) Link compile_commands.json for IDE support:
ln -s ./build/Debug/compile_commands.json ./compile_commands.json

# Run tests:
./build/Debug/runtests-api-postgresql-service --service-logs-pretty -vvx $PWD/tests
```
