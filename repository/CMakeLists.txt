cmake_minimum_required (VERSION 3.17.0)

include_directories(${CMAKE_BINARY_DIR}/common)

# if( BUILD_REPO_SERVER )
  # add_subdirectory( server )
  if( BUILD_RUST OR BUILD_REPO_SERVER )
    add_subdirectory( repo-server-rs )
  endif()
# endif()

# Ensure Rust build is triggered when BUILD_REPO_SERVER is enabled
if( BUILD_REPO_SERVER AND NOT BUILD_RUST )
  message(STATUS "BUILD_REPO_SERVER is enabled but BUILD_RUST is not. Enabling BUILD_RUST for repo-server-rs")
  set(BUILD_RUST TRUE)
endif()

if( BUILD_AUTHZ )
  add_subdirectory( gridftp )
endif()
