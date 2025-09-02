# CMakeLists.txt for repo-server-rs Rust project
# This file is included by the main CMakeLists.txt when BUILD_RUST is enabled

# Set project-specific variables
set(RUST_PROJECT_NAME "repo-server-rs")
set(RUST_CONFIG_FILE "repo-server.toml")

# Include the Rust configuration
include(${PROJECT_SOURCE_DIR}/cmake/Rust.cmake)

# Build the Rust project
build_rust_project(${RUST_PROJECT_NAME} ${DEPENDENCY_INSTALL_PATH})

# Install the Rust binary
install_rust_binary(${RUST_PROJECT_NAME})

# Add tests if enabled
if(BUILD_TESTS)
    add_rust_tests(${RUST_PROJECT_NAME} ${DEPENDENCY_INSTALL_PATH})
endif()

# Add run target
add_rust_run_target(${RUST_PROJECT_NAME} ${RUST_CONFIG_FILE} ${DEPENDENCY_INSTALL_PATH})

# Add clean target
clean_rust_build(${RUST_PROJECT_NAME})

# Set the binary path for parent scope
set(${RUST_PROJECT_NAME}_BINARY "${RUST_TARGET_DIR}/${RUST_PROJECT_NAME}" PARENT_SCOPE)

# Print project information
message("")
message("Rust Project: ${RUST_PROJECT_NAME}")
message("    Source Directory: ${CMAKE_CURRENT_SOURCE_DIR}")
message("    Binary Output: ${RUST_TARGET_DIR}/${RUST_PROJECT_NAME}")
message("    Configuration: ${RUST_CONFIG_FILE}")
message("")
