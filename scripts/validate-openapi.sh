#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
API_DIR="$PROJECT_DIR/docs/api"
ERRORS=0

echo "=== Conf-Ops OpenAPI Validation ==="
echo ""

# 1. Check redocly is available
if ! command -v npx &> /dev/null; then
    echo "ERROR: npx not found. Install Node.js first."
    exit 1
fi

# 2. Bundle to verify all $refs resolve (most important check)
echo "--- [1/5] Bundling to verify \$ref resolution ---"
BUNDLE_OUTPUT=$(mktemp /tmp/conf-ops-bundle-XXXXXX.yaml)
if npx @redocly/cli bundle "$API_DIR/openapi.yaml" -o "$BUNDLE_OUTPUT" 2>&1; then
    echo "PASS: All \$refs resolved successfully"
    BUNDLE_SIZE=$(wc -c < "$BUNDLE_OUTPUT" | tr -d ' ')
    echo "  Bundled size: ${BUNDLE_SIZE} bytes"
    rm -f "$BUNDLE_OUTPUT"
else
    echo "FAIL: \$ref resolution errors found"
    ERRORS=$((ERRORS + 1))
    rm -f "$BUNDLE_OUTPUT"
fi
echo ""

# 3. Lint OpenAPI spec (informational — nullable warnings are known)
echo "--- [2/5] Linting OpenAPI spec with Redocly ---"
LINT_OUTPUT=$(npx @redocly/cli lint "$API_DIR/openapi.yaml" --format=stylish 2>&1 || true)
# Count real errors (exclude nullable warnings which are OpenAPI 3.0 vs 3.1 style difference)
REAL_ERRORS=$(echo "$LINT_OUTPUT" | grep 'error' | grep -v 'nullable' | grep -v 'validated in' | grep -v 'Validation failed' || true)
if [ -z "$REAL_ERRORS" ]; then
    NULLABLE_COUNT=$(echo "$LINT_OUTPUT" | grep -c 'nullable' || true)
    echo "PASS: No structural errors (${NULLABLE_COUNT} nullable style warnings — OpenAPI 3.0 vs 3.1 convention)"
else
    echo "WARN: Lint found non-nullable issues:"
    echo "$REAL_ERRORS"
    ERRORS=$((ERRORS + 1))
fi
echo ""

# 4. Check operationId uniqueness
echo "--- [3/5] Checking operationId uniqueness ---"
OPIDS=$(grep -rh 'operationId:' "$API_DIR/paths/" 2>/dev/null | sed 's/.*operationId: *//' | sort)
DUPES=$(echo "$OPIDS" | uniq -d)
if [ -z "$DUPES" ]; then
    OPID_COUNT=$(echo "$OPIDS" | wc -l | tr -d ' ')
    echo "PASS: All $OPID_COUNT operationIds are unique"
else
    echo "FAIL: Duplicate operationIds found:"
    echo "$DUPES" | while read -r dup; do
        echo "  - $dup"
        grep -rn "operationId: *$dup" "$API_DIR/paths/"
    done
    ERRORS=$((ERRORS + 1))
fi
echo ""

# 5. Check tag consistency
echo "--- [4/5] Checking tag consistency ---"
# Extract declared tags from openapi.yaml
DECLARED_TAGS=$(grep '  - name: ' "$API_DIR/openapi.yaml" | sed 's/.*- name: //' | sed 's/"//g' | sort)
# Extract used tags from path files — only match lines directly under 'tags:' key
USED_TAGS=$(grep -rh '^\s*tags:' -A5 "$API_DIR/paths/" | grep '^\s*- [A-Z]' | sed 's/^\s*- //' | sed 's/"//g' | sort -u)
TAG_ERRORS=0
for tag in $USED_TAGS; do
    # Skip if it looks like a UUID or non-tag value
    if echo "$tag" | grep -qE '^[0-9a-f-]+$'; then
        continue
    fi
    if ! echo "$DECLARED_TAGS" | grep -q "^${tag}$"; then
        echo "  WARN: Tag '$tag' used in paths but not declared in openapi.yaml"
        TAG_ERRORS=$((TAG_ERRORS + 1))
    fi
done
if [ $TAG_ERRORS -eq 0 ]; then
    echo "PASS: All used tags are declared"
else
    ERRORS=$((ERRORS + 1))
fi
echo ""

# 6. Check pagination consistency
echo "--- [5/5] Checking list endpoint pagination ---"
PAGINATION_WARNS=0
grep -rn 'operationId: list' "$API_DIR/paths/" | while IFS=: read -r file line opid; do
    endpoint=$(echo "$opid" | sed 's/.*operationId: *//')
    # Check if cursor parameter exists nearby (within 40 lines)
    if ! sed -n "${line},$((line+40))p" "$file" | grep -q 'cursor'; then
        echo "  WARN: $endpoint in $(basename "$file") may be missing cursor pagination parameter"
    fi
done
echo "  (Check warnings above manually — some list endpoints may intentionally skip pagination)"
echo ""

# Summary
echo "=== Summary ==="
if [ $ERRORS -eq 0 ]; then
    echo "All checks passed!"
    exit 0
else
    echo "$ERRORS check(s) had issues. Review above output."
    exit 1
fi
