#!/bin/bash
# Run RNG test suites against randpass PRNG
# Results are saved to the project root

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
ARCH=$(uname -m)

cd "$PROJECT_DIR"

# Build the test binary
echo "Building rng_test binary..."
cargo build --release --bin rng_test

RNG_BIN="./target/release/rng_test"
BIGCRUSH_BIN="./bigcrush_wrapper"

run_bigcrush() {
    local TEST_TYPE="${1:-big}"

    if [ ! -x "$BIGCRUSH_BIN" ]; then
        echo "BigCrush wrapper not found. Build with:"
        echo "  gcc -O3 -o bigcrush_wrapper src/bin/bigcrush_wrapper.c -ltestu01 -lprobdist -lmylib -lm"
        return 1
    fi

    echo ""
    case "$TEST_TYPE" in
        small)
            echo "=== Running SmallCrush (~10 seconds) ==="
            OUTPUT_FILE="${PROJECT_DIR}/${ARCH}_smallcrush.txt"
            $RNG_BIN | $BIGCRUSH_BIN --small 2>&1 | tee "$OUTPUT_FILE"
            ;;
        medium)
            echo "=== Running Crush (~30 minutes) ==="
            OUTPUT_FILE="${PROJECT_DIR}/${ARCH}_crush.txt"
            $RNG_BIN | $BIGCRUSH_BIN --medium 2>&1 | tee "$OUTPUT_FILE"
            ;;
        big)
            echo "=== Running BigCrush (~4 hours) ==="
            OUTPUT_FILE="${PROJECT_DIR}/${ARCH}_bigcrush.txt"
            $RNG_BIN | $BIGCRUSH_BIN --big 2>&1 | tee "$OUTPUT_FILE"
            ;;
    esac

    echo ""
    echo "Results saved to: $OUTPUT_FILE"
}

run_dieharder() {
    echo ""
    echo "=== Running Dieharder (full battery) ==="
    echo "This takes ~20-30 minutes..."

    OUTPUT_FILE="${PROJECT_DIR}/${ARCH}_dieharder.txt"

    $RNG_BIN | dieharder -a -g 200 2>&1 | tee "$OUTPUT_FILE"

    # Add summary header
    PASSED=$(grep -c "PASSED" "$OUTPUT_FILE" || echo 0)
    WEAK=$(grep -c "WEAK" "$OUTPUT_FILE" || echo 0)
    FAILED=$(grep -c "FAILED" "$OUTPUT_FILE" || echo 0)
    TOTAL=$((PASSED + WEAK + FAILED))

    echo ""
    echo "Dieharder complete: $TOTAL tests, $PASSED passed, $WEAK weak, $FAILED failed"
    echo "Results saved to: $OUTPUT_FILE"
}

run_practrand() {
    local LIMIT="${1:-1TB}"

    if ! command -v RNG_test &> /dev/null; then
        echo "PractRand (RNG_test) not found. Install with:"
        echo "  ./scripts/install_practrand.sh"
        return 1
    fi

    echo ""
    echo "=== Running PractRand (limit: $LIMIT) ==="
    echo "This may take hours to days depending on limit..."

    OUTPUT_FILE="${PROJECT_DIR}/${ARCH}_practrand.txt"

    $RNG_BIN | RNG_test stdin -tlmax "$LIMIT" 2>&1 | tee "$OUTPUT_FILE"

    echo ""
    echo "PractRand complete. Results saved to: $OUTPUT_FILE"
}

case "${1:-all}" in
    dieharder)
        run_dieharder
        ;;
    practrand)
        run_practrand "${2:-1TB}"
        ;;
    smallcrush)
        run_bigcrush small
        ;;
    crush)
        run_bigcrush medium
        ;;
    bigcrush)
        run_bigcrush big
        ;;
    all)
        run_dieharder
        run_bigcrush big
        run_practrand "${2:-1TB}"
        ;;
    *)
        echo "Usage: $0 [dieharder|practrand|smallcrush|crush|bigcrush|all] [practrand_limit]"
        echo ""
        echo "Examples:"
        echo "  $0 dieharder           # Run dieharder only (~20 min)"
        echo "  $0 smallcrush          # Run TestU01 SmallCrush (~10 sec)"
        echo "  $0 crush               # Run TestU01 Crush (~30 min)"
        echo "  $0 bigcrush            # Run TestU01 BigCrush (~4 hours)"
        echo "  $0 practrand 1GB       # Run PractRand up to 1GB"
        echo "  $0 all 256GB           # Run all suites"
        exit 1
        ;;
esac
