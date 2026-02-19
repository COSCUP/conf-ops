# Conf-Ops Validation Scripts

This directory contains validation scripts for checking the consistency and correctness of the Conf-Ops specification files.

## Prerequisites

### For OpenAPI Validation
- Node.js (for `npx`)
- Redocly CLI (automatically installed via `npx`)

```bash
# No manual installation needed - npx will fetch @redocly/cli automatically
```

### For Cross-Reference Checks
- Python 3.7+
- PyYAML

```bash
pip install pyyaml
```

## Usage

### Quick Start

```bash
# Run all validations (recommended)
make validate

# Or run individual checks
make lint          # OpenAPI linting only
make bundle        # Bundle and verify $refs
make check-refs    # Cross-reference checks
make build-docs    # Generate HTML documentation
```

### Manual Execution

```bash
# Full validation suite
./scripts/validate-all.sh

# OpenAPI validation only
./scripts/validate-openapi.sh

# Cross-reference checks only
./scripts/check-cross-refs.py
```

## Scripts Overview

### `validate-openapi.sh`

OpenAPI specification validation with 5 checks:

1. **Linting** — Redocly CLI standard validation
2. **Bundling** — Verify all `$ref` resolve correctly
3. **operationId uniqueness** — No duplicate operation IDs
4. **Tag consistency** — All used tags are declared
5. **Pagination** — List endpoints have cursor parameters

**Exit code:** 0 if all checks pass, 1 if any check fails.

### `check-cross-refs.py`

Cross-document consistency checker with 3 checks:

1. **Entity Coverage** — Data model tables have corresponding API schemas
2. **Enum Consistency** — SQL ENUMs match API schema enums
3. **Environment Variables** — No naming inconsistencies across system docs

**Exit code:** 0 if no errors (warnings allowed), 1 if errors found.

### `validate-all.sh`

Wrapper script that runs both validation scripts and reports overall status.

**Exit code:** 0 if all phases pass, non-zero otherwise.

## Output Examples

### Success Output

```
╔══════════════════════════════════════════╗
║  Conf-Ops Specification Validation Suite ║
╚══════════════════════════════════════════╝

━━━ Phase 1: OpenAPI Validation ━━━
=== Conf-Ops OpenAPI Validation ===

--- [1/5] Linting OpenAPI spec with Redocly ---
PASS: Lint passed

--- [2/5] Bundling to verify $ref resolution ---
PASS: All $refs resolved successfully
  Bundled size: 156789 bytes

--- [3/5] Checking operationId uniqueness ---
PASS: All 87 operationIds are unique

--- [4/5] Checking tag consistency ---
PASS: All used tags are declared

--- [5/5] Checking list endpoint pagination ---
  (Check warnings above manually — some list endpoints may intentionally skip pagination)

=== Summary ===
All checks passed!

━━━ Phase 2: Cross-Reference Checks ━━━
=== Conf-Ops Cross-Reference Checker ===

--- [1/3] Entity Coverage ---
  Data model tables: 28
  API entity schemas: 25
  PASS: All main entities have API schemas

--- [2/3] Enum Consistency ---
  Data model ENUMs: 7
  API schema enums: 7
  PASS: project_status ↔ ProjectStatus values match
  PASS: task_status ↔ TaskStatus values match
  PASS: todo_status ↔ TodoStatus values match
  PASS: todo_type ↔ TodoType values match
  PASS: org_role ↔ OrgRole values match
  PASS: member_role ↔ MemberRole values match
  PASS: message_source_type ↔ MessageSourceType values match

--- [3/3] Environment Variable Consistency ---
  Total unique env vars found: 42
  PASS: No naming inconsistencies detected

=== Summary ===
All cross-reference checks passed!

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
All validations passed!
```

### Error Output Example

```
--- [3/5] Checking operationId uniqueness ---
FAIL: Duplicate operationIds found:
  - updateNotificationPreferences
    docs/api/paths/accounts.yaml:299:      operationId: updateNotificationPreferences
    docs/api/paths/notifications.yaml:236:      operationId: updateNotificationPreferences

=== Summary ===
1 check(s) had issues. Review above output.
```

## CI Integration

Add to `.github/workflows/validate-specs.yml`:

```yaml
name: Validate Specifications

on:
  pull_request:
    paths:
      - 'docs/**'
  push:
    branches:
      - main
      - v2

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - uses: actions/setup-python@v5
        with:
          python-version: '3.11'
      - name: Install dependencies
        run: |
          pip install pyyaml
      - name: Run validation
        run: make validate
```

## Troubleshooting

### `npx: command not found`
- Install Node.js from https://nodejs.org/

### `ModuleNotFoundError: No module named 'yaml'`
- Run `pip install pyyaml`

### Permission denied
- Run `chmod +x scripts/*.sh scripts/*.py`

### False positives in enum checks
- Update the `junction_tables` set in `check-cross-refs.py` to exclude known junction tables

## Future Improvements

Potential validation checks to add:

- [ ] Schema type consistency (SQL types vs OpenAPI types)
- [ ] x-permissions format validation
- [ ] Request/Response schema field correspondence
- [ ] Example data validation against schemas
- [ ] Discriminated union completeness checks
- [ ] Security scheme consistency
