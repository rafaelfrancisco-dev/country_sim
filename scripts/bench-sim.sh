#!/usr/bin/env sh

set -eu

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is not installed or not in PATH" >&2
  exit 1
fi

PROFILE="canonical"
SEED=""
COUNTRIES=""
POP_PER_COUNTRY=""
DAYS=""
THREADS=""
SIM_MODE=""
OUT_JSON=""
OUT_CSV=""
FLAMEGRAPH_OUT=""
TRACE_ENABLED="0"
THREAD_SWEEP="0"
TRACE_OUT_CSV=""
TRACE_SUMMARY_JSON=""

SEED_OVERRIDE=""
COUNTRIES_OVERRIDE=""
POP_PER_COUNTRY_OVERRIDE=""
DAYS_OVERRIDE=""
THREADS_OVERRIDE=""
SIM_MODE_OVERRIDE=""
OUT_JSON_OVERRIDE=""
OUT_CSV_OVERRIDE=""
TRACE_OUT_CSV_OVERRIDE=""
TRACE_SUMMARY_JSON_OVERRIDE=""

set_profile_defaults() {
  profile="$1"
  case "$profile" in
    quick)
      SEED="0x5eedc0de"
      COUNTRIES="2"
      POP_PER_COUNTRY="100000"
      DAYS="60"
      THREADS="8"
      SIM_MODE="max-speed"
      OUT_JSON="target/bench/quick.json"
      OUT_CSV="target/bench/quick.csv"
      TRACE_OUT_CSV="target/bench/quick-trace-days.csv"
      TRACE_SUMMARY_JSON="target/bench/quick-trace-summary.json"
      ;;
    canonical)
      SEED="0x5eedc0de"
      COUNTRIES="5"
      POP_PER_COUNTRY="1000000"
      DAYS="180"
      THREADS="8"
      SIM_MODE="max-speed"
      OUT_JSON="target/bench/latest.json"
      OUT_CSV="target/bench/latest.csv"
      TRACE_OUT_CSV="target/bench/trace-days.csv"
      TRACE_SUMMARY_JSON="target/bench/trace-summary.json"
      ;;
    *)
      echo "error: unknown benchmark profile '$profile' (expected quick|canonical)" >&2
      exit 1
      ;;
  esac
}

usage() {
  cat <<'EOF'
Usage:
  ./scripts/bench-sim.sh \
    --profile quick|canonical \
    --seed 0x5eedc0de \
    --countries 5 \
    --population-per-country 1000000 \
    --days 180 \
    --threads 8 \
    --sim-mode max-speed \
    --out-json target/bench/latest.json \
    --out-csv target/bench/latest.csv \
    [--trace-enabled] \
    [--thread-sweep] \
    [--trace-out-csv target/bench/trace-days.csv] \
    [--trace-summary-json target/bench/trace-summary.json] \
    [--flamegraph target/bench/flamegraph.svg]

Profiles:
  quick     2 countries × 100k population × 60 days (optimization loop)
  canonical 5 countries × 1,000,000 population × 180 days (final validation)
EOF
}

append_suffix() {
  path="$1"
  suffix="$2"
  case "$path" in
    *.*)
      base="${path%.*}"
      ext="${path##*.}"
      printf '%s_%s.%s' "$base" "$suffix" "$ext"
      ;;
    *)
      printf '%s_%s' "$path" "$suffix"
      ;;
  esac
}

run_benchmark_once() {
  run_days="$1"
  run_json="$2"
  run_csv="$3"
  run_trace_csv="$4"
  run_trace_summary="$5"

  mkdir -p "$(dirname "$run_json")" "$(dirname "$run_csv")"
  if [ -n "$run_trace_csv" ]; then
    mkdir -p "$(dirname "$run_trace_csv")"
  fi
  if [ -n "$run_trace_summary" ]; then
    mkdir -p "$(dirname "$run_trace_summary")"
  fi

  echo "Running release benchmark harness..."
  echo "Scenario: seed=$SEED countries=$COUNTRIES population_per_country=$POP_PER_COUNTRY days=$run_days"
  echo "Threads: $THREADS"
  echo "Outputs: json=$run_json csv=$run_csv"
  if [ "$TRACE_ENABLED" = "1" ]; then
    echo "Trace outputs: csv=$run_trace_csv summary=$run_trace_summary"
  fi

  if [ "$TRACE_ENABLED" = "1" ]; then
    BENCH_TRACE_ENABLED="1" \
    BENCH_THREAD_SWEEP="$THREAD_SWEEP" \
    BENCH_TRACE_OUTPUT_CSV="$run_trace_csv" \
    BENCH_TRACE_SUMMARY_JSON="$run_trace_summary" \
    BENCH_SEED="$SEED" \
    BENCH_COUNTRIES="$COUNTRIES" \
    BENCH_POP_PER_COUNTRY="$POP_PER_COUNTRY" \
    BENCH_DAYS="$run_days" \
    BENCH_THREADS="$THREADS" \
    BENCH_SIM_MODE="$SIM_MODE_NORMALIZED" \
    BENCH_OUTPUT_JSON="$run_json" \
    BENCH_OUTPUT_CSV="$run_csv" \
    RAYON_NUM_THREADS="$THREADS" \
    cargo bench --bench manual_world_benchmark --features manual-harness,bench-trace
  else
    BENCH_THREAD_SWEEP="$THREAD_SWEEP" \
    BENCH_SEED="$SEED" \
    BENCH_COUNTRIES="$COUNTRIES" \
    BENCH_POP_PER_COUNTRY="$POP_PER_COUNTRY" \
    BENCH_DAYS="$run_days" \
    BENCH_THREADS="$THREADS" \
    BENCH_SIM_MODE="$SIM_MODE_NORMALIZED" \
    BENCH_OUTPUT_JSON="$run_json" \
    BENCH_OUTPUT_CSV="$run_csv" \
    RAYON_NUM_THREADS="$THREADS" \
    cargo bench --bench manual_world_benchmark --features manual-harness
  fi
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --profile)
      PROFILE="$2"
      shift 2
      ;;
    --seed)
      SEED_OVERRIDE="$2"
      shift 2
      ;;
    --countries)
      COUNTRIES_OVERRIDE="$2"
      shift 2
      ;;
    --population-per-country)
      POP_PER_COUNTRY_OVERRIDE="$2"
      shift 2
      ;;
    --days)
      DAYS_OVERRIDE="$2"
      shift 2
      ;;
    --threads)
      THREADS_OVERRIDE="$2"
      shift 2
      ;;
    --sim-mode)
      SIM_MODE_OVERRIDE="$2"
      shift 2
      ;;
    --out-json)
      OUT_JSON_OVERRIDE="$2"
      shift 2
      ;;
    --out-csv)
      OUT_CSV_OVERRIDE="$2"
      shift 2
      ;;
    --trace-enabled)
      TRACE_ENABLED="1"
      shift 1
      ;;
    --thread-sweep)
      THREAD_SWEEP="1"
      shift 1
      ;;
    --trace-out-csv)
      TRACE_OUT_CSV_OVERRIDE="$2"
      shift 2
      ;;
    --trace-summary-json)
      TRACE_SUMMARY_JSON_OVERRIDE="$2"
      shift 2
      ;;
    --flamegraph)
      FLAMEGRAPH_OUT="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument '$1'" >&2
      usage
      exit 1
      ;;
  esac
done

PROFILE_NORMALIZED="$(printf '%s' "$PROFILE" | tr '[:upper:]' '[:lower:]' | tr '_' '-')"
set_profile_defaults "$PROFILE_NORMALIZED"

if [ -n "$SEED_OVERRIDE" ]; then
  SEED="$SEED_OVERRIDE"
fi
if [ -n "$COUNTRIES_OVERRIDE" ]; then
  COUNTRIES="$COUNTRIES_OVERRIDE"
fi
if [ -n "$POP_PER_COUNTRY_OVERRIDE" ]; then
  POP_PER_COUNTRY="$POP_PER_COUNTRY_OVERRIDE"
fi
if [ -n "$DAYS_OVERRIDE" ]; then
  DAYS="$DAYS_OVERRIDE"
fi
if [ -n "$THREADS_OVERRIDE" ]; then
  THREADS="$THREADS_OVERRIDE"
fi
if [ -n "$SIM_MODE_OVERRIDE" ]; then
  SIM_MODE="$SIM_MODE_OVERRIDE"
fi
if [ -n "$OUT_JSON_OVERRIDE" ]; then
  OUT_JSON="$OUT_JSON_OVERRIDE"
fi
if [ -n "$OUT_CSV_OVERRIDE" ]; then
  OUT_CSV="$OUT_CSV_OVERRIDE"
fi
if [ -n "$TRACE_OUT_CSV_OVERRIDE" ]; then
  TRACE_OUT_CSV="$TRACE_OUT_CSV_OVERRIDE"
fi
if [ -n "$TRACE_SUMMARY_JSON_OVERRIDE" ]; then
  TRACE_SUMMARY_JSON="$TRACE_SUMMARY_JSON_OVERRIDE"
fi

SIM_MODE_NORMALIZED="$(printf '%s' "$SIM_MODE" | tr '[:upper:]' '[:lower:]' | tr '_' '-')"
if [ "$SIM_MODE_NORMALIZED" != "max-speed" ]; then
  echo "error: --sim-mode must be max-speed for the benchmark protocol" >&2
  exit 1
fi

if [ "$TRACE_ENABLED" = "1" ] && [ -z "$TRACE_OUT_CSV" ]; then
  TRACE_OUT_CSV="$(append_suffix "$OUT_CSV" "trace-days")"
fi
if [ "$TRACE_ENABLED" = "1" ] && [ -z "$TRACE_SUMMARY_JSON" ]; then
  TRACE_SUMMARY_JSON="$(append_suffix "$OUT_JSON" "trace-summary")"
fi

run_benchmark_once "$DAYS" "$OUT_JSON" "$OUT_CSV" "$TRACE_OUT_CSV" "$TRACE_SUMMARY_JSON"

if [ "$TRACE_ENABLED" = "1" ] && [ "$DAYS" = "100" ] && [ -f "$TRACE_SUMMARY_JSON" ]; then
  if grep -Eq '"inconclusive"[[:space:]]*:[[:space:]]*true' "$TRACE_SUMMARY_JSON"; then
    fallback_days="300"
    fallback_json="$(append_suffix "$OUT_JSON" "300d")"
    fallback_csv="$(append_suffix "$OUT_CSV" "300d")"
    fallback_trace_csv="$(append_suffix "$TRACE_OUT_CSV" "300d")"
    fallback_trace_summary="$(append_suffix "$TRACE_SUMMARY_JSON" "300d")"
    echo "Trace summary was inconclusive for 100 days; running fallback ${fallback_days}-day benchmark."
    run_benchmark_once \
      "$fallback_days" \
      "$fallback_json" \
      "$fallback_csv" \
      "$fallback_trace_csv" \
      "$fallback_trace_summary"
  fi
fi

if [ -n "$FLAMEGRAPH_OUT" ]; then
  if ! cargo flamegraph --help >/dev/null 2>&1; then
    echo "error: cargo-flamegraph is not available." >&2
    echo "Install it with: cargo install flamegraph" >&2
    exit 1
  fi

  mkdir -p "$(dirname "$FLAMEGRAPH_OUT")"
  echo "Generating flamegraph at $FLAMEGRAPH_OUT..."
  if [ "$TRACE_ENABLED" = "1" ]; then
    BENCH_SEED="$SEED" \
    BENCH_COUNTRIES="$COUNTRIES" \
    BENCH_POP_PER_COUNTRY="$POP_PER_COUNTRY" \
    BENCH_DAYS="$DAYS" \
    BENCH_THREADS="$THREADS" \
    BENCH_SIM_MODE="$SIM_MODE_NORMALIZED" \
    BENCH_OUTPUT_JSON="$OUT_JSON" \
    BENCH_OUTPUT_CSV="$OUT_CSV" \
    BENCH_TRACE_ENABLED="$TRACE_ENABLED" \
    BENCH_THREAD_SWEEP="$THREAD_SWEEP" \
    BENCH_TRACE_OUTPUT_CSV="$TRACE_OUT_CSV" \
    BENCH_TRACE_SUMMARY_JSON="$TRACE_SUMMARY_JSON" \
    RAYON_NUM_THREADS="$THREADS" \
    cargo flamegraph \
      --bench manual_world_benchmark \
      --profile flamegraph \
      -o "$FLAMEGRAPH_OUT" \
      --features manual-harness,bench-trace
  else
    BENCH_SEED="$SEED" \
    BENCH_COUNTRIES="$COUNTRIES" \
    BENCH_POP_PER_COUNTRY="$POP_PER_COUNTRY" \
    BENCH_DAYS="$DAYS" \
    BENCH_THREADS="$THREADS" \
    BENCH_THREAD_SWEEP="$THREAD_SWEEP" \
    BENCH_SIM_MODE="$SIM_MODE_NORMALIZED" \
    BENCH_OUTPUT_JSON="$OUT_JSON" \
    BENCH_OUTPUT_CSV="$OUT_CSV" \
    RAYON_NUM_THREADS="$THREADS" \
    cargo flamegraph \
      --bench manual_world_benchmark \
      --profile flamegraph \
      -o "$FLAMEGRAPH_OUT" \
      --features manual-harness
  fi
fi

echo "Benchmark complete."
