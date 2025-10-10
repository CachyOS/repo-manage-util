FROM archlinux:base-devel as builder

# Install app dependencies
RUN pacman -Syu --noconfirm --noprogressbar --needed \
    git python cmake ninja clang llvm lld boost boost-libs libbsd tcl libxml2 util-linux zlib \
    sudo crypto++ yaml-cpp c-ares libev python-jinja \
    libnghttp2 postgresql meson pkg-config

# Add the repository key
RUN pacman-key --init && \
    pacman-key --recv-keys F3B607488DB35A47 --keyserver keyserver.ubuntu.com && \
    pacman-key --lsign-key F3B607488DB35A47
# Install cctz for userver
RUN pacman -U --noconfirm --noprogressbar \
    'https://mirror.cachyos.org/repo/x86_64/cachyos/cctz-2.5-1-x86_64.pkg.tar.zst' \
    'https://mirror.cachyos.org/repo/x86_64/cachyos/http-parser-2.9.4-2-x86_64.pkg.tar.zst'

# Create user builder
RUN useradd -d /app builder
RUN echo "builder ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Bundle app source
WORKDIR /app
COPY . .

USER builder

RUN sudo chown -R builder:builder .

# Install cctz for userver
#RUN git clone https://aur.archlinux.org/cctz.git && cd cctz && makepkg --rmdeps -sicf --cleanbuild --noconfirm --noprogressbar --nocheck

# Configure third-party
RUN ./run_with_clang.sh "./setup_libpq.py $PWD" && ./run_with_clang.sh "./setup_userver.py $PWD"

# Configure & Build
RUN ./configure.sh -t=Release --use_clang && ./build.sh

RUN sudo cp ./build/Release/api-postgresql-service ./{postgres_service.yaml,dynamic_config_fallback.json,secure_data.json,config_vars-docker.yaml} /
WORKDIR /
RUN sudo rm -rf /app
WORKDIR /app
RUN sudo cp /{api-postgresql-service,postgres_service.yaml,dynamic_config_fallback.json,secure_data.json,config_vars-docker.yaml} .

RUN sudo chown -R builder:builder .

# Production
#FROM nginx:stable-alpine as production
FROM archlinux:latest as production

# Bundle app source
WORKDIR /app
COPY --from=builder /app/* /app/
COPY --from=builder /usr/lib/libcctz.so* /usr/lib/
COPY --from=builder /usr/lib/libhttp_parser.so* /usr/lib/

RUN pacman -Syu --needed --noconfirm --noprogressbar \
    sudo boost-libs libbsd tcl libxml2 util-linux zlib \
    crypto++ yaml-cpp c-ares libev \
    postgresql

# Create user builder
RUN useradd -d /app builder
RUN echo "builder ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

USER builder
RUN sudo chown -R builder:builder .

EXPOSE 5862
CMD [ "/app/api-postgresql-service", "--config", "postgres_service.yaml", "--config_vars", "config_vars-docker.yaml"]
