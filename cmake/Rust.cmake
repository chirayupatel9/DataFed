# Rust.cmake - CMake configuration for Rust projects
# This file handles building and running Rust projects using Cargo

# Find Rust and Cargo
find_program(CARGO cargo)
if(NOT CARGO)
    message(FATAL_ERROR "Cargo not found. Please install Rust and Cargo.")
endif()

# Find Rust compiler
find_program(RUSTC rustc)
if(NOT RUSTC)
    message(FATAL_ERROR "Rust compiler not found. Please install Rust.")
endif()

# Set Rust project paths using CMake variables
set(RUST_PROJECT_DIR "${PROJECT_SOURCE_DIR}/repository/repo-server-rs")
set(RUST_BUILD_DIR "${CMAKE_BINARY_DIR}/rust")
set(RUST_TARGET_DIR "${RUST_BUILD_DIR}/target")

# Create build directory
file(MAKE_DIRECTORY ${RUST_BUILD_DIR})

# Set Rust build type
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(RUST_BUILD_TYPE "debug")
else()
    set(RUST_BUILD_TYPE "release")
endif()

# Set Rust target directory
set(RUST_TARGET_DIR "${RUST_BUILD_DIR}/target/${RUST_BUILD_TYPE}")
# Cargo sometimes creates an extra subdirectory, so we need to check both paths
set(RUST_BINARY_DIR "${RUST_BUILD_DIR}/target/${RUST_BUILD_TYPE}")
set(RUST_BINARY_DIR_ALT "${RUST_BUILD_DIR}/target/${RUST_BUILD_TYPE}/${RUST_BUILD_TYPE}")

# Function to build Rust project
function(build_rust_project PROJECT_NAME DEPENDENCY_PATH)
    set(RUST_PROJECT_PATH "${RUST_PROJECT_DIR}")
    
    # Set variables for build.rs.in template
    set(CMAKE_CURRENT_SOURCE_DIR "${RUST_PROJECT_DIR}")
    set(DEPENDENCY_INSTALL_PATH "${DEPENDENCY_PATH}")
    
    # Generate build.rs configuration file with CMake variables
    configure_file(
        "${RUST_PROJECT_DIR}/build.rs.in"
        "${RUST_PROJECT_DIR}/build.rs"
        @ONLY
    )
    
    # Create a custom target for building the Rust project
    add_custom_target(${PROJECT_NAME}-rust ALL
        COMMAND ${CMAKE_COMMAND} -E env
            CARGO_TARGET_DIR=${RUST_TARGET_DIR}
            LD_LIBRARY_PATH=${DEPENDENCY_PATH}/lib:$ENV{LD_LIBRARY_PATH}
            ${CARGO} build --${RUST_BUILD_TYPE}
        WORKING_DIRECTORY ${RUST_PROJECT_PATH}
        COMMENT "Building Rust project ${PROJECT_NAME}"
        VERBATIM
    )
    
    # Set output binary path - check both possible locations
    if(EXISTS "${RUST_BINARY_DIR_ALT}/${PROJECT_NAME}")
        set(${PROJECT_NAME}_BINARY "${RUST_BINARY_DIR_ALT}/${PROJECT_NAME}" PARENT_SCOPE)
    else()
        set(${PROJECT_NAME}_BINARY "${RUST_BINARY_DIR}/${PROJECT_NAME}" PARENT_SCOPE)
    endif()
    
    # Add dependency on common library if it exists
    if(TARGET common)
        add_dependencies(${PROJECT_NAME}-rust common)
    endif()
endfunction()

# Function to install Rust binary
function(install_rust_binary PROJECT_NAME)
    set(RUST_BINARY "${RUST_TARGET_DIR}/${PROJECT_NAME}")
    
    # Install the binary
    install(TARGETS ${PROJECT_NAME}-rust
        RUNTIME DESTINATION ${DATAFED_INSTALL_PATH}/bin
    )
    
    # Also install the binary file directly
    install(FILES ${RUST_BINARY}
        DESTINATION ${DATAFED_INSTALL_PATH}/bin
        PERMISSIONS OWNER_READ OWNER_WRITE OWNER_EXECUTE
                   GROUP_READ GROUP_EXECUTE
                   WORLD_READ WORLD_EXECUTE
    )
endfunction()

# Function to create test target for Rust project
function(add_rust_tests PROJECT_NAME DEPENDENCY_PATH)
    if(BUILD_TESTS AND ENABLE_RUST_TESTS)
        add_custom_target(${PROJECT_NAME}-test
            COMMAND ${CMAKE_COMMAND} -E env
                CARGO_TARGET_DIR=${RUST_TARGET_DIR}
                LD_LIBRARY_PATH=${DEPENDENCY_PATH}/lib:$ENV{LD_LIBRARY_PATH}
                ${CARGO} test
            WORKING_DIRECTORY ${RUST_PROJECT_DIR}
            COMMENT "Running tests for Rust project ${PROJECT_NAME}"
            VERBATIM
        )
        
        # Add to CTest if enabled
        if(ENABLE_TESTING)
            add_test(NAME ${PROJECT_NAME}_rust_tests
                     COMMAND ${CARGO} test
                     WORKING_DIRECTORY ${RUST_PROJECT_DIR})
            set_tests_properties(${PROJECT_NAME}_rust_tests
                PROPERTIES ENVIRONMENT "CARGO_TARGET_DIR=${RUST_TARGET_DIR};LD_LIBRARY_PATH=${DEPENDENCY_PATH}/lib:$ENV{LD_LIBRARY_PATH}")
        endif()
    endif()
endfunction()

# Function to create run target for Rust project
function(add_rust_run_target PROJECT_NAME CONFIG_FILE DEPENDENCY_PATH)
    # Check both possible binary locations
    if(EXISTS "${RUST_BINARY_DIR_ALT}/${PROJECT_NAME}")
        set(RUST_BINARY "${RUST_BINARY_DIR_ALT}/${PROJECT_NAME}")
    else()
        set(RUST_BINARY "${RUST_BINARY_DIR}/${PROJECT_NAME}")
    endif()
    
    set(CONFIG_PATH "${RUST_PROJECT_DIR}/${CONFIG_FILE}")
    
    # Create a run target
    add_custom_target(${PROJECT_NAME}-run
        COMMAND ${CMAKE_COMMAND} -E env
            LD_LIBRARY_PATH=${DEPENDENCY_PATH}/lib:$ENV{LD_LIBRARY_PATH}
            ${RUST_BINARY} --cfg ${CONFIG_PATH}
        DEPENDS ${PROJECT_NAME}-rust
        COMMENT "Running Rust project ${PROJECT_NAME}"
        VERBATIM
    )
    
    # Make it non-default
    set_target_properties(${PROJECT_NAME}-run PROPERTIES EXCLUDE_FROM_ALL TRUE)
endfunction()

# Function to clean Rust build artifacts
function(clean_rust_build PROJECT_NAME)
    add_custom_target(${PROJECT_NAME}-clean
        COMMAND ${CARGO} clean
        WORKING_DIRECTORY ${RUST_PROJECT_DIR}
        COMMENT "Cleaning Rust build artifacts for ${PROJECT_NAME}"
        VERBATIM
    )
    
    # Make it non-default
    set_target_properties(${PROJECT_NAME}-clean PROPERTIES EXCLUDE_FROM_ALL TRUE)
endfunction()



# Set Rust environment variables
set(ENV{CARGO_TARGET_DIR} ${RUST_TARGET_DIR})

# Print Rust configuration
message("")
message("Rust Configuration")
message("    Rust Compiler: ${RUSTC}")
message("    Cargo: ${CARGO}")
message("    Rust Project Dir: ${RUST_PROJECT_DIR}")
message("    Rust Build Dir: ${RUST_BUILD_DIR}")
message("    Rust Target Dir: ${RUST_TARGET_DIR}")
message("    Rust Build Type: ${RUST_BUILD_TYPE}")
message("")
