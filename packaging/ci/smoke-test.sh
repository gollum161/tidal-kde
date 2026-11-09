#!/bin/bash
# Starts the app without a window for a few seconds. Fails if it exits early
# or prints QML/library loading errors (e.g. a missing Kirigami module).
# Usage: smoke-test.sh <command> [args...]
set -uo pipefail

log="$(mktemp)"
# Qt logs to the journal when stderr is no terminal; force stderr so errors are visible.
export QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software QT_FORCE_STDERR_LOGGING=1
export HOME="$(mktemp -d)" XDG_RUNTIME_DIR="$(mktemp -d)"

timeout 15 "$@" >"$log" 2>&1
rc=$?
cat "$log"

if [ "$rc" -ne 124 ]; then
	echo "FAIL: the app exited after less than 15 s (exit code $rc)" >&2
	exit 1
fi
if grep -Eiq 'is not installed|cannot load library|failed to load|QQmlApplicationEngine|Type .* unavailable|could not (find|load) the Qt platform plugin' "$log"; then
	echo "FAIL: QML or library loading errors (see above)" >&2
	exit 1
fi
echo "OK: the app started and kept running"
