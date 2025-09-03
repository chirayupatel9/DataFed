#!/bin/bash

set -euf -o pipefail
echo "=== ENTRYPOINT SCRIPT STARTING ==="
echo "Arguments: $@"
SCRIPT=$(realpath "$0")
SOURCE=$(dirname "$SCRIPT")
PROJECT_ROOT=$(realpath "${SOURCE}/../..")
echo "Script: $SCRIPT"
echo "Source: $SOURCE"
echo "Project root: $PROJECT_ROOT"

"${PROJECT_ROOT}/scripts/generate_datafed.sh"
"${PROJECT_ROOT}/scripts/generate_repo_config.sh"
"${PROJECT_ROOT}/scripts/install_repo.sh"

# This is only part of the solution the other part is running chown
if [ -n "$UID" ] && [ "$UID" != "0" ]; then
	echo "Switching datafed user to UID: ${UID}"
	usermod -u "$UID" datafed
	chown -R datafed:root "${PROJECT_ROOT}"
	chown -R datafed:root "${DATAFED_INSTALL_PATH}/repo/"
	# Make sure the folder exists
	mkdir -p "${DATAFED_GCS_COLLECTION_ROOT_PATH}/${DATAFED_REPO_ID_AND_DIR}"
	chown -R datafed:root "${DATAFED_GCS_COLLECTION_ROOT_PATH}/${DATAFED_REPO_ID_AND_DIR}"
elif [ "$UID" = "0" ]; then
	echo "UID is 0 (root), skipping usermod"
fi

log_path="${DATAFED_DEFAULT_LOG_PATH:-/var/log/datafed}"

if [ ! -d "${log_path}" ]; then
	su -c "mkdir -p ${log_path}" datafed
fi

if [ ! -f "${DATAFED_INSTALL_PATH}/keys/datafed-core-key.pub" ]; then
	echo "datafed-core-key.pub not found, downloading from the core server"
	wget --no-check-certificate "https://${DATAFED_DOMAIN}/datafed-core-key.pub" -P "${DATAFED_INSTALL_PATH}/keys/"
fi

datafed_repo_exec=$(basename "$1")
echo "Attempting to run: $1"
echo "Executable name: ${datafed_repo_exec}"

# Check if the executable exists and is executable
if [ -f "$1" ]; then
	echo "Executable exists: $1"
	if [ -x "$1" ]; then
		echo "Executable is executable: $1"
	else
		echo "Executable is not executable: $1"
	fi
else
	echo "Executable does not exist: $1"
fi

# Function to test if an executable can run (for glibc compatibility check)
test_executable() {
	local exe="$1"
	if [ -f "$exe" ] && [ -x "$exe" ]; then
		# Try to run the executable with --help to test if it can start
		if timeout 5s "$exe" --help >/dev/null 2>&1; then
			return 0
		else
			echo "Executable $exe failed to run (likely glibc compatibility issue)"
			return 1
		fi
	fi
	return 1
}

# Determine which server to run
rust_server="/opt/datafed/repo/repo-server-rs"
cpp_server="/opt/datafed/repo/datafed-repo"

# If the requested executable is the Rust server, test it first
if [ "${datafed_repo_exec}" = "repo-server-rs" ]; then
	echo "Testing Rust server compatibility..."
	if test_executable "$rust_server"; then
		echo "Rust server is compatible, using Rust server"
		# Replace the executable path with the full path
		set -- "$rust_server" "${@:2}"
	else
		echo "Rust server failed compatibility test, falling back to C++ server"
		# Replace the executable path with the C++ server
		set -- "$cpp_server" "${@:2}"
		datafed_repo_exec="datafed-repo"
	fi
fi

if [ "${datafed_repo_exec}" = "datafed-repo" ]; then
	# Send output to log file
	# For this to work all commands must be passed in as a single string
	echo "Running with logging to datafed-repo.log"
	su datafed -c '"$@"' -- argv0 "$@" 2>&1 | tee -a "$log_path/datafed-repo.log"
else
	echo "Not sending output to datafed-core.log"
	# If not do not by default send to log file
	echo "Running without logging"
	su datafed -c '"$@"' -- argv0 "$@"
fi

# Allow the container to exist for a bit in case we need to jump in and look
# around
echo "Container sleeping"
sleep 10000
echo "Container exiting after sleep"
