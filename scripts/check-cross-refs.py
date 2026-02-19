#!/usr/bin/env python3
"""Cross-reference checker for Conf-Ops spec files.

Checks consistency between:
- Data model (.md) column types vs API schema types
- Environment variable naming across system docs
- Entity coverage (data model entities have corresponding API schemas)
"""
import os
import re
import sys
import yaml
from pathlib import Path
from collections import defaultdict

PROJECT_DIR = Path(__file__).parent.parent
DOCS_DIR = PROJECT_DIR / "docs"
API_DIR = DOCS_DIR / "api"
DATA_MODEL_DIR = DOCS_DIR / "data-model"
SYSTEM_DIR = DOCS_DIR / "system"

errors = []
warnings = []

def check_entity_coverage():
    """Check that data model entities have corresponding API schemas."""
    print("--- [1/3] Entity Coverage ---")

    # Get data model entities from filenames
    dm_files = sorted(DATA_MODEL_DIR.glob("*.md"))
    dm_entities = set()
    for f in dm_files:
        if f.name == "README.md":
            continue
        # Extract entity name from file content (look for CREATE TABLE)
        content = f.read_text()
        tables = re.findall(r'CREATE TABLE (\w+)', content)
        dm_entities.update(tables)

    # Get API schema entities
    entities_file = API_DIR / "schemas" / "entities.yaml"
    if entities_file.exists():
        with open(entities_file) as f:
            entities_yaml = yaml.safe_load(f)
        api_entities = set()
        if entities_yaml:
            for key in entities_yaml:
                # Strip 'Response' suffix to get base entity name
                base = re.sub(r'Response$', '', key)
                if base:
                    api_entities.add(base.lower())
    else:
        api_entities = set()

    # Compare
    # Known junction/internal tables that don't need API schemas
    junction_tables = {'member_tag_assignments', 'todo_assignees', 'task_template_tags',
                     'crdt_operations', 'memory_versions', 'library_document_versions',
                     'web_push_subscriptions', 'magic_link_tokens', 'passkey_credentials',
                     'refresh_tokens', 'webhook_event_logs', 'organization_members',
                     'conversation_states', 'last_seen_positions', 'scheduled_reminders'}

    for table in sorted(dm_entities):
        if table in junction_tables:
            continue
        # Skip partition tables (e.g., audit_logs_2025_01)
        if re.match(r'.+_\d{4}_\d{2}$', table):
            continue
        # Normalize: snake_case table → no underscores, handle plurals
        normalized = table.lower().replace('_', '')
        # Generate singular variants (handle regular and irregular plurals)
        variants = {normalized}
        if normalized.endswith('ies'):
            variants.add(normalized[:-3] + 'y')  # memories → memory
        if normalized.endswith('ries'):
            variants.add(normalized[:-3] + 'ry')  # entries → entry
        if normalized.endswith('es'):
            variants.add(normalized[:-2])
        if normalized.endswith('s'):
            variants.add(normalized[:-1])
        found = False
        for api_ent in api_entities:
            api_norm = api_ent.lower().replace('_', '')
            if api_norm in variants or api_norm + 's' == normalized or api_norm + 'es' == normalized:
                found = True
                break
        if not found:
            warnings.append(f"Table '{table}' has no matching API Response schema")

    print(f"  Data model tables: {len(dm_entities)}")
    print(f"  API entity schemas: {len(api_entities)}")
    missing = len([w for w in warnings if 'no matching API' in w])
    if missing:
        print(f"  Missing schemas: {missing} (see warnings)")
    else:
        print("  PASS: All main entities have API schemas")
    print()

def check_enum_consistency():
    """Check that ENUM types in data model match API schema enums."""
    print("--- [2/3] Enum Consistency ---")

    # Extract ENUMs from data model files
    dm_enums = {}
    for f in sorted(DATA_MODEL_DIR.glob("*.md")):
        content = f.read_text()
        # Match CREATE TYPE ... AS ENUM (...)
        for match in re.finditer(r"CREATE TYPE (\w+) AS ENUM \(([^)]+)\)", content):
            name = match.group(1)
            values = [v.strip().strip("'\"") for v in match.group(2).split(',')]
            dm_enums[name] = {'values': values, 'file': f.name}
        # Also match VARCHAR with CHECK or comments listing values

    # Extract enums from common.yaml
    common_file = API_DIR / "schemas" / "common.yaml"
    api_enums = {}
    if common_file.exists():
        with open(common_file) as f:
            common_yaml = yaml.safe_load(f)
        if common_yaml:
            for key, schema in common_yaml.items():
                if isinstance(schema, dict) and 'enum' in schema:
                    api_enums[key] = schema['enum']

    print(f"  Data model ENUMs: {len(dm_enums)}")
    print(f"  API schema enums: {len(api_enums)}")

    # Cross-reference by trying to match names
    for dm_name, dm_info in dm_enums.items():
        # Try to find matching API enum (e.g., task_status → TaskStatus)
        camel = ''.join(word.capitalize() for word in dm_name.split('_'))
        if camel in api_enums:
            dm_vals = set(dm_info['values'])
            api_vals = set(api_enums[camel])
            if dm_vals != api_vals:
                errors.append(f"Enum mismatch: {dm_name} ({dm_info['file']}) = {dm_vals}, API {camel} = {api_vals}")
            else:
                print(f"  PASS: {dm_name} ↔ {camel} values match")
        else:
            warnings.append(f"Data model ENUM '{dm_name}' has no matching API enum (expected '{camel}')")
    print()

def check_env_vars():
    """Check environment variable naming consistency across system docs."""
    print("--- [3/3] Environment Variable Consistency ---")

    env_vars = defaultdict(list)  # var_name -> [(file, line)]

    for f in sorted(SYSTEM_DIR.glob("*.md")):
        content = f.read_text()
        for i, line in enumerate(content.split('\n'), 1):
            # Match common env var patterns
            for match in re.finditer(r'`([A-Z][A-Z_0-9]{2,})`', line):
                var = match.group(1)
                # Skip common non-env-var patterns
                if var in ('NOT', 'NULL', 'TRUE', 'FALSE', 'AND', 'ENUM', 'CREATE',
                          'TABLE', 'TYPE', 'VARCHAR', 'JSONB', 'BYTEA', 'TIMESTAMPTZ',
                          'UUID', 'INET', 'TEXT', 'DEFAULT', 'INDEX', 'PRIMARY', 'KEY',
                          'REFERENCES', 'DELETE', 'CASCADE', 'SET', 'CHECK', 'UNIQUE',
                          'INSERT', 'UPDATE', 'SELECT', 'WHERE', 'FROM', 'JOIN', 'LEFT',
                          'GET', 'POST', 'PUT', 'PATCH', 'HEAD', 'OPTIONS',
                          'HMAC', 'SHA256', 'RBAC', 'CRDT', 'SMTP', 'HTTP',
                          'HTTPS', 'WSS', 'TCP', 'MCP', 'SSE', 'STDIO', 'JSON', 'YAML',
                          'API', 'URL', 'URI', 'JWT', 'JWK', 'VAPID', 'DNS', 'TLS', 'SSL'):
                    continue
                env_vars[var].append((f.name, i))

    # Find potential duplicates/inconsistencies
    # Group by base concept (strip prefix)
    concepts = defaultdict(list)
    for var in env_vars:
        # Remove common prefixes to find the concept
        base = re.sub(r'^(AUTH_|APP_|LLM_|AI_|CRDT_|EMAIL_|INBOUND_|DB_|CACHE_|STORAGE_|WS_)', '', var)
        concepts[base].append(var)

    inconsistent = 0
    for base, variants in concepts.items():
        if len(variants) > 1:
            # Check if they're actually different vars or naming inconsistencies
            prefixes = set(var.replace(base, '').rstrip('_') for var in variants)
            if len(prefixes) > 1:
                files_info = []
                for var in variants:
                    locs = [f"{f}:{l}" for f, l in env_vars[var]]
                    files_info.append(f"  {var}: {', '.join(locs)}")
                warnings.append(f"Potential env var inconsistency for concept '{base}':\n" + '\n'.join(files_info))
                inconsistent += 1

    total_vars = len(env_vars)
    print(f"  Total unique env vars found: {total_vars}")
    if inconsistent:
        print(f"  Potential inconsistencies: {inconsistent} (see warnings)")
    else:
        print("  PASS: No naming inconsistencies detected")
    print()

def main():
    print("=== Conf-Ops Cross-Reference Checker ===\n")

    check_entity_coverage()
    check_enum_consistency()
    check_env_vars()

    # Summary
    print("=== Summary ===")
    if errors:
        print(f"\nERRORS ({len(errors)}):")
        for e in errors:
            print(f"  ✗ {e}")
    if warnings:
        print(f"\nWARNINGS ({len(warnings)}):")
        for w in warnings:
            print(f"  ⚠ {w}")

    if not errors and not warnings:
        print("All cross-reference checks passed!")
        return 0
    elif errors:
        return 1
    else:
        return 0

if __name__ == "__main__":
    sys.exit(main())
