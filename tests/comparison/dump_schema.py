#!/usr/bin/env python3
"""
XSD Schema Dumper - Generates normalized JSON from XSD using Python xmlschema.

This script loads an XSD schema using the Python xmlschema library and outputs
a normalized JSON representation that the Rust xmlschema-rs implementation
should match exactly.

Usage:
    python dump_schema.py <schema.xsd> [--catalog <catalog.xml>] [--output <output.json>]

Output Format:
    {
        "target_namespace": "...",
        "schema_location": "...",
        "element_form_default": "qualified" | "unqualified",
        "root_elements": [...],
        "complex_types": [...],
        "simple_types": [...]
    }
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Optional


def create_uri_mapper(catalog_path: str):
    """Create a URI mapper function for resolving URNs to local file paths."""
    from urllib.parse import unquote

    urn_map = {}
    current_base = None
    catalog_dir = Path(catalog_path).parent

    with open(catalog_path, 'r') as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('--'):
                continue

            if line.startswith('BASE'):
                parts = line.split('"')
                if len(parts) >= 2:
                    current_base = parts[1]
                    # Strip leading ../../ from BASE paths
                    if current_base.startswith('../../'):
                        current_base = current_base[6:]

            elif line.startswith('URI'):
                parts = line.split('"')
                if len(parts) >= 4:
                    urn = parts[1]
                    local_path = parts[3]
                    if current_base:
                        full_path = catalog_dir / current_base / local_path
                    else:
                        full_path = catalog_dir / local_path
                    urn_map[urn] = str(full_path.resolve())

    def uri_mapper(uri: str) -> str:
        if uri is None:
            return None
        decoded_uri = unquote(uri)
        resolved = urn_map.get(decoded_uri) or urn_map.get(uri)
        return resolved if resolved else uri

    return uri_mapper


def extract_type_info(type_obj) -> dict:
    """Extract detailed info about a type."""
    info = {
        'name': type_obj.qualified_name if hasattr(type_obj, 'qualified_name') else None,
        'qualified_name': type_obj.qualified_name if hasattr(type_obj, 'qualified_name') else None,
        'category': type(type_obj).__name__,
        'is_complex': 'Complex' in type(type_obj).__name__,
        'is_simple': 'Simple' in type(type_obj).__name__,
    }

    # For complex types, get content model
    if hasattr(type_obj, 'content') and type_obj.content:
        info['content_model'] = type(type_obj.content).__name__

    # Get attributes
    if hasattr(type_obj, 'attributes') and type_obj.attributes:
        info['attributes'] = sorted([
            {
                'name': attr.name or 'unknown',
                'type': (attr.type.qualified_name
                        if hasattr(attr.type, 'qualified_name') and attr.type.qualified_name
                        else str(attr.type)) if attr.type else 'xs:string',
                'use': attr.use if hasattr(attr, 'use') and attr.use else 'optional',
                'default': attr.default if hasattr(attr, 'default') else None,
            }
            for attr in type_obj.attributes.values()
        ], key=lambda x: x['name'])

    # Get elements for complex types
    if hasattr(type_obj, 'content') and type_obj.content and hasattr(type_obj.content, 'iter_elements'):
        try:
            elements = list(type_obj.content.iter_elements())
            if elements:
                info['child_elements'] = [
                    {
                        'name': elem.qualified_name or elem.name or 'unknown',
                        'type': (elem.type.qualified_name
                                if hasattr(elem.type, 'qualified_name') and elem.type.qualified_name
                                else type(elem.type).__name__) if elem.type else 'unknown',
                        'min_occurs': elem.min_occurs if hasattr(elem, 'min_occurs') else 1,
                        'max_occurs': elem.max_occurs if hasattr(elem, 'max_occurs') else 1,
                    }
                    for elem in elements
                ]
        except Exception:
            pass

    return info


def extract_element_info(element) -> dict:
    """Extract detailed info about an element."""
    return {
        'name': element.qualified_name or element.name,
        'qualified_name': element.qualified_name or element.name,
        'type': extract_type_info(element.type) if element.type else None,
        'min_occurs': element.min_occurs if hasattr(element, 'min_occurs') else 1,
        'max_occurs': element.max_occurs if hasattr(element, 'max_occurs') else 1,
        'nillable': element.nillable if hasattr(element, 'nillable') else False,
        'default': element.default if hasattr(element, 'default') else None,
    }


def extract_complex_type_info(type_obj) -> dict:
    """Extract info for complex type definition."""
    info = {
        'name': type_obj.qualified_name or type_obj.name,
        'qualified_name': type_obj.qualified_name or type_obj.name,
        'category': type(type_obj).__name__,
        'is_complex': True,
        'is_simple': False,
    }

    if hasattr(type_obj, 'content') and type_obj.content:
        info['content_model'] = type(type_obj.content).__name__

    if hasattr(type_obj, 'attributes') and type_obj.attributes:
        info['attributes'] = sorted([
            {
                'name': attr.name or 'unknown',
                'type': (attr.type.qualified_name
                        if hasattr(attr.type, 'qualified_name') and attr.type.qualified_name
                        else str(attr.type)) if attr.type else 'xs:string',
                'use': attr.use if hasattr(attr, 'use') and attr.use else 'optional',
                'default': attr.default if hasattr(attr, 'default') else None,
            }
            for attr in type_obj.attributes.values()
        ], key=lambda x: x['name'])

    if hasattr(type_obj, 'content') and type_obj.content and hasattr(type_obj.content, 'iter_elements'):
        try:
            elements = list(type_obj.content.iter_elements())
            if elements:
                info['child_elements'] = [
                    {
                        'name': elem.qualified_name or elem.name or 'unknown',
                        'type': (elem.type.qualified_name
                                if hasattr(elem.type, 'qualified_name') and elem.type.qualified_name
                                else type(elem.type).__name__) if elem.type else 'unknown',
                        'min_occurs': elem.min_occurs if hasattr(elem, 'min_occurs') else 1,
                        'max_occurs': elem.max_occurs if hasattr(elem, 'max_occurs') else 1,
                    }
                    for elem in elements
                ]
        except Exception:
            pass

    return info


def extract_simple_type_info(type_obj) -> dict:
    """Extract info for simple type definition."""
    info = {
        'name': type_obj.qualified_name or type_obj.name,
        'qualified_name': type_obj.qualified_name or type_obj.name,
        'category': type(type_obj).__name__,
        'base_type': type_obj.base_type.qualified_name if hasattr(type_obj, 'base_type') and type_obj.base_type else None,
    }

    # Extract facets/restrictions
    restrictions = []
    if hasattr(type_obj, 'facets'):
        facets = type_obj.facets
        if facets:
            for facet_name, facet_value in facets.items():
                if facet_name == 'enumeration' and facet_value:
                    restrictions.append({
                        'kind': 'Enumeration',
                        'values': list(facet_value)
                    })
                elif facet_name == 'pattern' and facet_value:
                    restrictions.append({
                        'kind': 'Pattern',
                        'value': str(facet_value)
                    })
                elif facet_name == 'minLength' and facet_value is not None:
                    restrictions.append({
                        'kind': 'MinLength',
                        'value': int(facet_value.value) if hasattr(facet_value, 'value') else int(facet_value)
                    })
                elif facet_name == 'maxLength' and facet_value is not None:
                    restrictions.append({
                        'kind': 'MaxLength',
                        'value': int(facet_value.value) if hasattr(facet_value, 'value') else int(facet_value)
                    })
                elif facet_name == 'length' and facet_value is not None:
                    restrictions.append({
                        'kind': 'Length',
                        'value': int(facet_value.value) if hasattr(facet_value, 'value') else int(facet_value)
                    })

    if restrictions:
        info['restrictions'] = restrictions

    return info


def dump_schema(schema_path: str, catalog_path: Optional[str] = None) -> dict:
    """Load an XSD schema and dump it as a normalized dictionary."""
    import xmlschema

    # Load schema with optional catalog
    if catalog_path:
        uri_mapper = create_uri_mapper(catalog_path)
        schema = xmlschema.XMLSchema(schema_path, uri_mapper=uri_mapper)
    else:
        schema = xmlschema.XMLSchema(schema_path)

    result = {
        'target_namespace': schema.target_namespace,
        'schema_location': schema.url if hasattr(schema, 'url') else None,
        'element_form_default': schema.element_form_default if hasattr(schema, 'element_form_default') else None,
        'root_elements': [],
        'complex_types': [],
        'simple_types': [],
    }

    # Extract root elements
    if hasattr(schema, 'elements') and schema.elements:
        for name, element in sorted(schema.elements.items()):
            result['root_elements'].append(extract_element_info(element))

    # Extract named types
    if hasattr(schema, 'types') and schema.types:
        for name, type_obj in sorted(schema.types.items()):
            type_name = type(type_obj).__name__
            if 'Complex' in type_name:
                result['complex_types'].append(extract_complex_type_info(type_obj))
            elif 'Simple' in type_name or 'Atomic' in type_name or 'List' in type_name or 'Union' in type_name:
                result['simple_types'].append(extract_simple_type_info(type_obj))

    return result


def main():
    parser = argparse.ArgumentParser(
        description='Dump XSD schema to normalized JSON for comparison testing'
    )
    parser.add_argument('schema', help='Path to XSD schema file')
    parser.add_argument('--catalog', '-c', help='Path to XML catalog file')
    parser.add_argument('--output', '-o', help='Output JSON file (default: stdout)')
    parser.add_argument('--pretty', '-p', action='store_true', help='Pretty print JSON')

    args = parser.parse_args()

    try:
        result = dump_schema(args.schema, args.catalog)

        indent = 2 if args.pretty else None
        json_output = json.dumps(result, indent=indent, sort_keys=True)

        if args.output:
            with open(args.output, 'w') as f:
                f.write(json_output)
        else:
            print(json_output)

    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
