#!/usr/bin/env bash
set -euo pipefail

MODE="${1:?usage: check-coverage.sh <line|branch>}"

case "${MODE}" in
  line)
    exec cargo llvm-cov --workspace --summary-only --fail-under-lines 85
    ;;
  branch)
    if ! rustup toolchain list | grep -q '^nightly'; then
      cat >&2 <<'EOF'
nightly Rust is required for branch coverage.
Install it with: rustup toolchain install nightly --profile minimal
EOF
      exit 1
    fi

    if ! command -v grcov >/dev/null 2>&1; then
      cat >&2 <<'EOF'
grcov is required for branch coverage (workaround for nightly llvm-cov SIGSEGV).
Install it with: cargo install grcov
EOF
      exit 1
    fi

    # Step 1: Run tests under nightly with branch instrumentation, skip the
    # broken llvm-cov report/export step.
    cargo +nightly llvm-cov --workspace --branch --no-report

    # Step 2: Use grcov to produce branch coverage from the raw profraw files,
    # working around the upstream LLVM 22.x SIGSEGV in llvm-cov report/export
    # when processing multiple object files with branch instrumentation.
    branch_pct="$(
      grcov target/llvm-cov-target/ \
        --binary-path target/llvm-cov-target/debug/ \
        -s . \
        --branch \
        --ignore-not-existing \
        --ignore '*/tests/*' --ignore '*/examples/*' --ignore '*/benches/*' \
        --ignore '*/.cargo/*' --ignore '*/.rustup/*' \
        -t lcov 2>/dev/null \
      | awk '
        /^BRF:/ { total += substr($0, 5) }
        /^BRH:/ { hit += substr($0, 5) }
        END {
          if (total > 0) printf "%.2f", (hit/total)*100
          else { print "0" > "/dev/stderr"; exit 1 }
        }'
    )"

    echo "branch coverage: ${branch_pct}%"

    if ! awk -v pct="${branch_pct}" 'BEGIN { exit !(pct + 0 >= 85) }'; then
      echo "branch coverage ${branch_pct}% is below the required 85%" >&2
      exit 1
    fi
    ;;
  *)
    echo "unsupported coverage mode: ${MODE}" >&2
    exit 1
    ;;
esac
