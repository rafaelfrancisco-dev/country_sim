#!/usr/bin/env sh

set -eu

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is not installed or not in PATH" >&2
  exit 1
fi

SEED="0x5eedc0de"
COUNTRIES="5"
POP_PER_COUNTRY="1000000"
DAYS="180"
THREADS="8"
SIM_MODE="max-speed"
OUT_JSON="target/bench/latest.json"
OUT_CSV="target/bench/latest.csv"
FLAMEGRAPH_OUT=""

usage() {
  cat <<'EOF'
Usage:
  ./scripts/bench-sim.sh \
    --seed 0x5eedc0de \
    --countries 5 \
    --population-per-country 1000000 \
    --days 180 \
    --threads 8 \
    --sim-mode max-speed \
    --out-json target/bench/latest.json \
    --out-csv target/bench/latest.csv \
    [--flamegraph target/bench/flamegraph.svg]
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --seed)
      SEED="$2"
      shift 2
      ;;
    --countries)
      COUNTRIES="$2"
      shift 2
      ;;
    --population-per-country)
      POP_PER_COUNTRY="$2"
      shift 2
      ;;
    --days)
      DAYS="$2"
      shift 2
      ;;
    --threads)
      THREADS="$2"
      shift 2
      ;;
    --sim-mode)
      SIM_MODE="$2"
      shift 2
      ;;
    --out-json)
      OUT_JSON="$2"
      shift 2
      ;;
    --out-csv)
      OUT_CSV="$2"
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

SIM_MODE_NORMALIZED="$(printf '%s' "$SIM_MODE" | tr '[:upper:]' '[:lower:]' | tr '_' '-')"
if [ "$SIM_MODE_NORMALIZED" != "max-speed" ]; then
  echo "error: --sim-mode must be max-speed for the benchmark protocol" >&2
  exit 1
fi

mkdir -p "$(dirname "$OUT_JSON")" "$(dirname "$OUT_CSV")"

echo "Running release benchmark harness..."
echo "Scenario: seed=$SEED countries=$COUNTRIES population_per_country=$POP_PER_COUNTRY days=$DAYS"
echo "Threads: $THREADS"
echo "Outputs: json=$OUT_JSON csv=$OUT_CSV"

BENCH_SEED="$SEED" \
BENCH_COUNTRIES="$COUNTRIES" \
BENCH_POP_PER_COUNTRY="$POP_PER_COUNTRY" \
BENCH_DAYS="$DAYS" \
BENCH_THREADS="$THREADS" \
BENCH_SIM_MODE="$SIM_MODE_NORMALIZED" \
BENCH_OUTPUT_JSON="$OUT_JSON" \
BENCH_OUTPUT_CSV="$OUT_CSV" \
RAYON_NUM_THREADS="$THREADS" \
cargo test --release benchmark_release_protocol_emits_metrics -- --ignored --nocapture

if [ -n "$FLAMEGRAPH_OUT" ]; then
  if ! cargo flamegraph --help >/dev/null 2>&1; then
    echo "error: cargo-flamegraph is not available." >&2
    echo "Install it with: cargo install flamegraph" >&2
    exit 1
  fi

  mkdir -p "$(dirname "$FLAMEGRAPH_OUT")"
  echo "Generating flamegraph at $FLAMEGRAPH_OUT..."
  BENCH_SEED="$SEED" \
  BENCH_COUNTRIES="$COUNTRIES" \
  BENCH_POP_PER_COUNTRY="$POP_PER_COUNTRY" \
  BENCH_DAYS="$DAYS" \
  BENCH_THREADS="$THREADS" \
  BENCH_SIM_MODE="$SIM_MODE_NORMALIZED" \
  BENCH_OUTPUT_JSON="$OUT_JSON" \
  BENCH_OUTPUT_CSV="$OUT_CSV" \
  RAYON_NUM_THREADS="$THREADS" \
  cargo flamegraph \
    --unit-test country_sim \
    --profile flamegraph \
    -o "$FLAMEGRAPH_OUT" \
    -- benchmark_release_protocol_emits_metrics --ignored --nocapture
fi

echo "Benchmark complete."
