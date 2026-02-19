#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TOTAL_ERRORS=0

echo "╔══════════════════════════════════════════╗"
echo "║  Conf-Ops Specification Validation Suite ║"
echo "╚══════════════════════════════════════════╝"
echo ""

# 1. OpenAPI validation
echo "━━━ Phase 1: OpenAPI Validation ━━━"
if bash "$SCRIPT_DIR/validate-openapi.sh"; then
    echo ""
else
    TOTAL_ERRORS=$((TOTAL_ERRORS + 1))
    echo ""
fi

# 2. Cross-reference checks
echo "━━━ Phase 2: Cross-Reference Checks ━━━"
if python3 "$SCRIPT_DIR/check-cross-refs.py"; then
    echo ""
else
    TOTAL_ERRORS=$((TOTAL_ERRORS + 1))
    echo ""
fi

# Final summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ $TOTAL_ERRORS -eq 0 ]; then
    echo "All validations passed!"
    exit 0
else
    echo "$TOTAL_ERRORS validation phase(s) had issues."
    exit 1
fi
