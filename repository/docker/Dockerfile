# NOTE this image must be built with respect to the base of the project i.e.
# cd ${PROJECT_ROOT} or cd DataFed
# docker build -f repository/docker/Dockerfile .

ARG           BUILD_BASE="debian:bookworm-slim"
ARG         DEPENDENCIES="dependencies"
ARG              RUNTIME="runtime"
ARG          DATAFED_DIR="/datafed"
ARG DATAFED_INSTALL_PATH="/opt/datafed"
ARG DATAFED_DEPENDENCIES_INSTALL_PATH="/opt/datafed/dependencies"
ARG            GCS_IMAGE="code.ornl.gov:4567/dlsw/datafed/gcs-ubuntu-focal"
ARG            BUILD_DIR="$DATAFED_DIR/source"
ARG              LIB_DIR="/usr/local/lib"

FROM ${DEPENDENCIES} AS repo-build

SHELL ["/bin/bash", "-c"]
ARG DATAFED_DIR
ARG BUILD_DIR
ARG DATAFED_INSTALL_PATH
ARG DATAFED_DEPENDENCIES_INSTALL_PATH

ENV DATAFED_INSTALL_PATH="$DATAFED_INSTALL_PATH"
# This port is needed to communicate with the DataFed core server
EXPOSE 7512
# Not quite sure what 9000 is doing that 7512 isn't, difference between egress
# and ingress?
EXPOSE 9000

COPY ./repository/CMakeLists.txt               ${BUILD_DIR}/repository/CMakeLists.txt
COPY ./CMakeLists.txt                          ${BUILD_DIR}
COPY ./scripts/dependency_versions.sh          ${BUILD_DIR}/scripts/
COPY ./scripts/dependency_install_functions.sh ${BUILD_DIR}/scripts/
COPY ./scripts/generate_datafed.sh             ${BUILD_DIR}/scripts/
COPY ./scripts/generate_repo_config.sh         ${BUILD_DIR}/scripts/
COPY ./scripts/install_repo.sh                 ${BUILD_DIR}/scripts/
COPY ./cmake                                   ${BUILD_DIR}/cmake
COPY ./repository/server                       ${BUILD_DIR}/repository/server
RUN rm -rf ${BUILD_DIR}/repository/repo-server-rs
COPY ./repository/repo-server-rs               ${BUILD_DIR}/repository/repo-server-rs

# Rust is already installed in the dependencies image
# Verify Rust installation and source code
RUN echo "=== Verifying Rust installation ===" && \
    rustc --version && \
    cargo --version && \
    echo "=== Checking Rust source code ===" && \
    ls -la ${BUILD_DIR}/repository/repo-server-rs/ && \
    echo "=== Rust source files ===" && \
    find ${BUILD_DIR}/repository/repo-server-rs/ -name "*.rs" -o -name "Cargo.toml" | head -10

RUN ${BUILD_DIR}/scripts/generate_datafed.sh && \
	${DATAFED_DEPENDENCIES_INSTALL_PATH}/bin/cmake -S. -B build						\
	-DBUILD_REPO_SERVER=True		\
	-DBUILD_AUTHZ=False					\
	-DBUILD_CORE_SERVER=False		\
	-DBUILD_WEB_SERVER=False		\
	-DBUILD_DOCS=False					\
	-DBUILD_PYTHON_CLIENT=False	\
	-DBUILD_FOXX=False				\
	-DBUILD_RUST=True
RUN echo "=== CMake configuration completed ===" && \
    echo "=== Checking if Rust targets were created ===" && \
    find build -name "*rust*" -o -name "*repo-server-rs*" 2>/dev/null || echo "No Rust targets found" && \
    echo "=== Checking CMake targets ===" && \
    ${DATAFED_DEPENDENCIES_INSTALL_PATH}/bin/cmake --build build --target help | grep -i rust || echo "No Rust targets found in CMake" && \
    echo "=== CMake build starting ==="
RUN ${DATAFED_DEPENDENCIES_INSTALL_PATH}/bin/cmake --build build -j 8
RUN echo "=== CMake build completed ===" && \
    echo "=== Checking build results ===" && \
    find build -name "*repo-server-rs*" -type f 2>/dev/null || echo "No repo-server-rs binaries found in build" && \
    echo "=== CMake install starting ==="
RUN ${DATAFED_DEPENDENCIES_INSTALL_PATH}/bin/cmake --build build --target install

# Debug: Check what files were actually created
RUN echo "=== Checking build output ===" && \
    find ${DATAFED_INSTALL_PATH} -name "*repo-server-rs*" -type f 2>/dev/null || echo "No repo-server-rs files found in ${DATAFED_INSTALL_PATH}" && \
    find build -name "*repo-server-rs*" -type f 2>/dev/null || echo "No repo-server-rs files found in build directory" && \
    find . -name "*repo-server-rs*" -type f 2>/dev/null || echo "No repo-server-rs files found in current directory"

# Create a simple script to copy Rust binaries if they exist
RUN echo '#!/bin/bash' > /tmp/copy_rust.sh && \
    echo 'set -e' >> /tmp/copy_rust.sh && \
    echo 'if [ -f /opt/datafed/repo/repo-server-rs ]; then' >> /tmp/copy_rust.sh && \
    echo '    echo "Copying repo-server-rs from repo directory"' >> /tmp/copy_rust.sh && \
    echo '    cp /opt/datafed/repo/repo-server-rs /tmp/repo-server-rs-repo' >> /tmp/copy_rust.sh && \
    echo 'fi' >> /tmp/copy_rust.sh && \
    echo 'if [ -f /opt/datafed/bin/repo-server-rs ]; then' >> /tmp/copy_rust.sh && \
    echo '    echo "Copying repo-server-rs from bin directory"' >> /tmp/copy_rust.sh && \
    echo '    cp /opt/datafed/bin/repo-server-rs /tmp/repo-server-rs-bin' >> /tmp/copy_rust.sh && \
    echo 'fi' >> /tmp/copy_rust.sh && \
    chmod +x /tmp/copy_rust.sh && \
    /tmp/copy_rust.sh

# Note: Boost libraries are built as static libraries and linked into the executables
# No need to copy shared libraries since we're using static linking

FROM ${RUNTIME} AS repo

SHELL ["/bin/bash", "-c"]
ARG DATAFED_DIR
ARG DATAFED_INSTALL_PATH
ARG BUILD_DIR
ARG LIB_DIR
ARG DATAFED_DEPENDENCIES_INSTALL_PATH

# The above should also be available at runtime
ENV              DATAFED_INSTALL_PATH="$DATAFED_INSTALL_PATH"
ENV DATAFED_DEPENDENCIES_INSTALL_PATH="${DATAFED_DEPENDENCIES_INSTALL_PATH}"
ENV  DATAFED_GCS_COLLECTION_BASE_PATH="/mnt/datafed"
ENV  DATAFED_GCS_COLLECTION_ROOT_PATH="/mnt/datafed"
ENV           DATAFED_REPO_ID_AND_DIR="home"
ENV                       DATAFED_DIR="$DATAFED_DIR"
ENV                         BUILD_DIR="$BUILD_DIR"
ENV                           LIB_DIR="$LIB_DIR"
ENV                    DATAFED_DOMAIN="datafed.ornl.gov"
ENV           DATAFED_DEFAULT_LOG_PATH="/var/log/datafed"

WORKDIR /datafed

# Note: Boost libraries are statically linked into the executables
# No need to copy shared libraries to runtime stage

USER datafed

COPY --chown=datafed:root ./scripts/generate_datafed.sh          ${DATAFED_DIR}/scripts/generate_datafed.sh
COPY --chown=datafed:root ./scripts/generate_repo_config.sh      ${DATAFED_DIR}/scripts/generate_repo_config.sh
COPY --chown=datafed:root ./scripts/install_repo.sh              ${DATAFED_DIR}/scripts/install_repo.sh
COPY --chown=datafed:root ./cmake/Version.cmake                  ${DATAFED_DIR}/cmake/Version.cmake
COPY --chown=datafed:root ./repository/docker/entrypoint_repo.sh ${BUILD_DIR}/repository/entrypoint.sh
COPY --chown=datafed:root ./repository/repo-server-rs/repo-server.toml ${DATAFED_INSTALL_PATH}/repo/repo-server.toml
COPY --from=repo-build --chown=datafed:root ${DATAFED_INSTALL_PATH}/repo/datafed-repo ${DATAFED_INSTALL_PATH}/repo/datafed-repo
COPY --from=repo-build --chown=datafed:root ${DATAFED_INSTALL_PATH}/repo/repo-server-rs ${DATAFED_INSTALL_PATH}/repo/repo-server-rs

USER root

RUN chown -R datafed:root /datafed
RUN chown -R datafed:root /opt


ENTRYPOINT [ "/datafed/source/repository/entrypoint.sh", "/opt/datafed/repo/repo-server-rs","--cfg","/opt/datafed/repo/repo-server.toml"]