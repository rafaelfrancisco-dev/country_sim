#!/usr/bin/env sh

set -eu

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is not installed or not in PATH" >&2
  exit 1
fi

BIN_NAME="${BIN_NAME:-country-sim}"
FLAMEGRAPH_DEBUG_SYMBOLS="${FLAMEGRAPH_DEBUG_SYMBOLS:-1}"
OUTPUT="${OUTPUT:-flamegraph.svg}"
MODE="${MODE:-auto}"

is_truthy() {
  case "$1" in
    1|true|TRUE|yes|YES|on|ON) return 0 ;;
    *) return 1 ;;
  esac
}

if [ "${PROFILE:-}" = "" ]; then
  if is_truthy "${FLAMEGRAPH_DEBUG_SYMBOLS}"; then
    PROFILE="flamegraph"
  else
    PROFILE="release"
  fi
else
  PROFILE="${PROFILE}"
fi

run_cargo_mode() {
  if ! cargo flamegraph --help >/dev/null 2>&1; then
    echo "error: cargo-flamegraph is not available." >&2
    echo "Install it with: cargo install flamegraph" >&2
    exit 1
  fi

  echo "Running ${BIN_NAME} with cargo flamegraph..."
  echo "Profile: ${PROFILE}"
  echo "Output file: ${OUTPUT}"
  if [ "${PROFILE}" = "release" ]; then
    echo "Tip: set FLAMEGRAPH_DEBUG_SYMBOLS=1 (uses profile=flamegraph) for symbolized stacks."
  fi
  cargo flamegraph --bin "${BIN_NAME}" --profile "${PROFILE}" -o "${OUTPUT}" -- "$@"
}

run_interactive_macos_mode() {
  if ! command -v flamegraph >/dev/null 2>&1; then
    echo "error: flamegraph is not available." >&2
    echo "Install it with: cargo install flamegraph" >&2
    exit 1
  fi

  existing_pid="$(pgrep -n -x "${BIN_NAME}" || true)"

  (
    i=0
    while :; do
      pid="$(pgrep -n -x "${BIN_NAME}" || true)"
      if [ -n "${pid}" ] && [ "${pid}" != "${existing_pid}" ]; then
        break
      fi
      i=$((i + 1))
      if [ "${i}" -gt 600 ]; then
        echo "error: timed out waiting for ${BIN_NAME} process" >&2
        exit 1
      fi
      sleep 0.1
    done

    echo "Attaching flamegraph to PID ${pid}..."
    flamegraph --pid "${pid}" -o "${OUTPUT}"
  ) &
  profiler_pid=$!

  echo "Launching ${BIN_NAME} interactively (macOS attach mode)..."
  echo "Profile: ${PROFILE}"
  echo "Quit the game normally (e.g. 'q') to finish profiling."
  echo "Output file: ${OUTPUT}"
  if [ "${PROFILE}" = "release" ]; then
    echo "Tip: set FLAMEGRAPH_DEBUG_SYMBOLS=1 (uses profile=flamegraph) for symbolized stacks."
  fi

  set +e
  cargo run --bin "${BIN_NAME}" --profile "${PROFILE}" -- "$@"
  app_status=$?
  set -e

  wait "${profiler_pid}" || true
  return "${app_status}"
}

if [ "${MODE}" = "auto" ]; then
  case "$(uname -s)" in
    Darwin) MODE="interactive-macos" ;;
    *) MODE="cargo" ;;
  esac
fi

case "${MODE}" in
  cargo)
    run_cargo_mode
    ;;
  interactive-macos)
    run_interactive_macos_mode
    ;;
  *)
    echo "error: invalid MODE '${MODE}'. Use MODE=auto|cargo|interactive-macos" >&2
    exit 1
    ;;
esac
