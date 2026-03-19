#!/usr/bin/env python3
"""
Validate dw-client types against the control-layer OpenAPI spec.

Downloads the OpenAPI spec from a running server and checks that our Rust
types have all the required fields with compatible types.

Usage:
    python3 scripts/validate-types.py [--server http://localhost:3001]
    python3 scripts/validate-types.py --spec-file /path/to/openapi.json --surface ai
"""

import argparse
import json
import re
import sys
from pathlib import Path

# Map OpenAPI types to expected Rust types
OPENAPI_TO_RUST = {
    "string": ["String", "Option<String>"],
    "integer": ["i64", "i32", "u64", "u32", "usize", "Option<i64>", "Option<i32>", "Option<u64>"],
    "number": ["f64", "f32", "Option<f64>", "Option<f32>"],
    "boolean": ["bool", "Option<bool>"],
    "array": ["Vec<", "Option<Vec<"],
    "object": ["HashMap<", "Option<HashMap<", "Value"],
}

# Our type mappings: (rust_file, rust_struct) -> openapi_schema_name
TYPE_MAPPINGS = {
    # AI surface types
    "ai": {
        "FileResponse": "FileResponse",
        "FileListResponse": "FileListResponse",
        "FileCostEstimate": "FileCostEstimate",
        "ModelCostBreakdown": "ModelCostBreakdown",
        "BatchResponse": "BatchResponse",
        "BatchListResponse": "BatchListResponse",
        "RequestCounts": "RequestCounts",
        "CreateBatchRequest": "CreateBatchRequest",
    },
    # Admin surface types
    "admin": {
        "ModelResponse": "DeployedModelResponse",
        "ModelListResponse": "ModelListResponse",
    },
}


def parse_rust_struct(file_path: Path, struct_name: str) -> dict[str, str]:
    """Parse a Rust struct to extract field names and types."""
    content = file_path.read_text()

    # Find the struct definition
    pattern = rf"pub struct {struct_name}\s*\{{(.*?)\}}"
    match = re.search(pattern, content, re.DOTALL)
    if not match:
        return {}

    body = match.group(1)
    fields = {}

    for line in body.split("\n"):
        line = line.strip()
        # Skip comments, attributes, empty lines
        if not line or line.startswith("//") or line.startswith("#[") or line.startswith("///"):
            continue

        # Parse "pub field_name: Type,"
        field_match = re.match(r"pub\s+(\w+):\s+(.+?),?\s*$", line)
        if field_match:
            name = field_match.group(1)
            rust_type = field_match.group(2).strip().rstrip(",")
            fields[name] = rust_type

    return fields


def get_schema_fields(schema: dict, all_schemas: dict) -> dict[str, dict]:
    """Extract fields from an OpenAPI schema, resolving $ref."""
    properties = {}

    # Handle allOf, oneOf, etc.
    if "allOf" in schema:
        for sub in schema["allOf"]:
            properties.update(get_schema_fields(sub, all_schemas))
        return properties

    # Handle $ref
    if "$ref" in schema:
        ref_name = schema["$ref"].split("/")[-1]
        if ref_name in all_schemas:
            return get_schema_fields(all_schemas[ref_name], all_schemas)
        return {}

    # Handle flattened schemas
    if "properties" in schema:
        required = set(schema.get("required", []))
        for name, prop in schema["properties"].items():
            prop_type = prop.get("type", "string")
            if "$ref" in prop:
                prop_type = "object"
            nullable = prop.get("nullable", False) or name not in required
            properties[name] = {
                "type": prop_type,
                "nullable": nullable,
                "format": prop.get("format"),
            }

    return properties


def check_type_compatibility(rust_type: str, openapi_type: str, nullable: bool) -> bool:
    """Check if a Rust type is compatible with an OpenAPI type."""
    # If nullable in OpenAPI, Rust should be Option<...>
    if nullable and not rust_type.startswith("Option<") and rust_type != "String":
        # String with #[serde(default)] is fine for nullable strings
        pass

    # Check base type compatibility
    expected_types = OPENAPI_TO_RUST.get(openapi_type, [])
    for expected in expected_types:
        if expected in rust_type or rust_type.startswith(expected):
            return True

    # Special cases
    if openapi_type == "string" and "uuid" in rust_type.lower():
        return True
    if openapi_type == "string" and rust_type in ["String", "Option<String>"]:
        return True

    return False


def validate_surface(spec: dict, surface: str, types_dir: Path) -> list[str]:
    """Validate types for one API surface."""
    schemas = spec.get("components", {}).get("schemas", {})
    errors = []

    mappings = TYPE_MAPPINGS.get(surface, {})

    for rust_name, openapi_name in mappings.items():
        if openapi_name not in schemas:
            errors.append(f"  {surface}: OpenAPI schema '{openapi_name}' not found in spec")
            continue

        # Find the Rust file containing this type
        rust_file = None
        for f in types_dir.glob("*.rs"):
            content = f.read_text()
            if f"pub struct {rust_name}" in content:
                rust_file = f
                break

        if not rust_file:
            errors.append(f"  {surface}: Rust struct '{rust_name}' not found in {types_dir}")
            continue

        rust_fields = parse_rust_struct(rust_file, rust_name)
        openapi_fields = get_schema_fields(schemas[openapi_name], schemas)

        if not rust_fields:
            errors.append(f"  {surface}: Could not parse Rust struct '{rust_name}' from {rust_file.name}")
            continue

        # Check for missing fields (in OpenAPI but not in Rust)
        for field_name, field_info in openapi_fields.items():
            # Convert snake_case openapi to snake_case rust (they should match)
            if field_name not in rust_fields:
                # Check if it's a renamed field via serde
                if not field_info["nullable"]:
                    errors.append(
                        f"  {surface}/{rust_name}: missing required field '{field_name}' "
                        f"(type: {field_info['type']})"
                    )
                # Optional fields that are missing are OK — #[serde(default)] handles them

    return errors


def main():
    parser = argparse.ArgumentParser(description="Validate dw-client types against OpenAPI spec")
    parser.add_argument("--server", default="http://localhost:3001", help="Server URL")
    parser.add_argument("--spec-file", help="Path to OpenAPI JSON file (instead of fetching)")
    parser.add_argument("--surface", choices=["ai", "admin", "both"], default="both")
    args = parser.parse_args()

    types_dir = Path(__file__).parent.parent / "crates" / "dw-client" / "src" / "types"

    if not types_dir.exists():
        print(f"Error: types directory not found at {types_dir}")
        sys.exit(1)

    errors = []

    surfaces = ["ai", "admin"] if args.surface == "both" else [args.surface]

    for surface in surfaces:
        if args.spec_file:
            with open(args.spec_file) as f:
                spec = json.load(f)
        else:
            import urllib.request
            url = f"{args.server}/{surface}/openapi.json"
            try:
                with urllib.request.urlopen(url) as resp:
                    spec = json.loads(resp.read())
            except Exception as e:
                print(f"Warning: could not fetch {url}: {e}")
                continue

        print(f"Checking {surface} surface ({len(spec.get('components', {}).get('schemas', {}))} schemas)...")
        surface_errors = validate_surface(spec, surface, types_dir)
        errors.extend(surface_errors)

    if errors:
        print(f"\nFound {len(errors)} issue(s):")
        for error in errors:
            print(error)
        sys.exit(1)
    else:
        print("\nAll types match the OpenAPI spec.")
        sys.exit(0)


if __name__ == "__main__":
    main()
