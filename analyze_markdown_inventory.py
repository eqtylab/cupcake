#!/usr/bin/env python3
"""
Analyze markdown file inventory for Cupcake documentation.

This script reads the markdown_inventory.json file and provides various
analyses about the documentation structure, age, and organization.

If the inventory file doesn't exist, it will be generated automatically.
"""

import json
import os
from datetime import datetime
from collections import defaultdict
from pathlib import Path


def generate_inventory(repo_root=None):
    """Generate markdown inventory by scanning the repository."""
    if repo_root is None:
        repo_root = Path(__file__).parent
    else:
        repo_root = Path(repo_root)

    print(f"Generating markdown inventory from {repo_root}...")

    # Find all markdown files
    markdown_files = sorted(repo_root.glob('**/*.md'))

    # Filter out excluded directories
    exclude_dirs = {'.git', 'node_modules', 'target', '.cupcake'}
    markdown_files = [
        f for f in markdown_files
        if not any(part.startswith('.') or part in exclude_dirs
                  for part in f.relative_to(repo_root).parts)
    ]

    def categorize_path(path_str):
        """Categorize markdown file by directory."""
        if path_str.startswith('docs/'):
            return 'documentation'
        elif path_str.startswith('eval/') or path_str.startswith('examples/'):
            return 'examples'
        elif '/' not in path_str:
            return 'root'
        else:
            return 'code'

    def get_file_info(filepath):
        """Extract metadata from markdown file."""
        stat = filepath.stat()
        rel_path = str(filepath.relative_to(repo_root))

        # Use birthtime if available (macOS), otherwise ctime
        created = datetime.fromtimestamp(
            stat.st_birthtime if hasattr(stat, 'st_birthtime') else stat.st_ctime
        )
        modified = datetime.fromtimestamp(stat.st_mtime)

        return {
            'path': rel_path,
            'size_bytes': stat.st_size,
            'created': created.strftime('%Y-%m-%d %H:%M:%S'),
            'modified': modified.strftime('%Y-%m-%d %H:%M:%S'),
            'category': categorize_path(rel_path)
        }

    inventory = {
        'generated_at': datetime.now().strftime('%Y-%m-%d %H:%M:%S'),
        'total_files': len(markdown_files),
        'files': [get_file_info(f) for f in markdown_files]
    }

    print(f"Found {len(markdown_files)} markdown files")
    return inventory


def load_inventory(filepath='markdown_inventory.json'):
    """Load the markdown inventory JSON file, generating it if needed."""
    filepath = Path(filepath)

    if not filepath.exists():
        print(f"Inventory file not found: {filepath}")
        inventory = generate_inventory()

        # Save the generated inventory
        with open(filepath, 'w') as f:
            json.dump(inventory, f, indent=2)
        print(f"Saved inventory to {filepath}\n")

        return inventory

    with open(filepath, 'r') as f:
        return json.load(f)


def print_header(text):
    """Print a formatted section header."""
    print(f"\n{'=' * 80}")
    print(f"  {text}")
    print(f"{'=' * 80}\n")


def analyze_by_category(data):
    """Analyze files grouped by category."""
    print_header("FILES BY CATEGORY")

    by_category = defaultdict(list)
    for file in data['files']:
        by_category[file['category']].append(file)

    # Print summary
    for category in sorted(by_category.keys()):
        files = by_category[category]
        total_size = sum(f['size_bytes'] for f in files)
        print(f"{category:20} {len(files):3} files  {total_size/1024:8.1f} KB")

    print(f"\n{'TOTAL':20} {data['total_files']:3} files")

    return by_category


def analyze_by_age(data):
    """Analyze files by creation and modification dates."""
    print_header("FILES BY AGE")

    # Parse dates
    for file in data['files']:
        file['created_dt'] = datetime.strptime(file['created'], '%Y-%m-%d %H:%M:%S')
        file['modified_dt'] = datetime.strptime(file['modified'], '%Y-%m-%d %H:%M:%S')

    # Sort by creation date
    by_created = sorted(data['files'], key=lambda x: x['created_dt'])

    print("OLDEST FILES (by creation):")
    for file in by_created[:10]:
        print(f"  {file['created']}  {file['path']}")

    print("\nNEWEST FILES (by creation):")
    for file in by_created[-10:]:
        print(f"  {file['created']}  {file['path']}")

    # Recently modified
    by_modified = sorted(data['files'], key=lambda x: x['modified_dt'])

    print("\nRECENTLY MODIFIED:")
    for file in by_modified[-10:]:
        print(f"  {file['modified']}  {file['path']}")


def analyze_stale_files(data):
    """Find files that haven't been modified in a while."""
    print_header("POTENTIALLY STALE FILES")

    now = datetime.now()

    # Files older than 30 days
    old_files = []
    for file in data['files']:
        modified_dt = datetime.strptime(file['modified'], '%Y-%m-%d %H:%M:%S')
        days_old = (now - modified_dt).days
        if days_old > 30:
            old_files.append((days_old, file))

    old_files.sort(key=lambda x: x[0], reverse=True)

    print(f"Files not modified in 30+ days: {len(old_files)}")
    print("\nOLDEST (by last modification):")
    for days, file in old_files[:15]:
        print(f"  {days:3} days  {file['modified']}  {file['path']}")


def analyze_documentation_structure(by_category):
    """Analyze the documentation structure."""
    print_header("DOCUMENTATION STRUCTURE")

    if 'documentation' in by_category:
        docs = by_category['documentation']

        # Group by subdirectory
        by_subdir = defaultdict(list)
        for doc in docs:
            path_parts = doc['path'].split('/')
            if len(path_parts) > 2:
                subdir = '/'.join(path_parts[:2])
            else:
                subdir = 'docs/'
            by_subdir[subdir].append(doc)

        print("Documentation subdirectories:")
        for subdir in sorted(by_subdir.keys()):
            files = by_subdir[subdir]
            print(f"\n  {subdir}/ ({len(files)} files)")
            for file in sorted(files, key=lambda x: x['path']):
                size_kb = file['size_bytes'] / 1024
                print(f"    - {Path(file['path']).name:50} {size_kb:6.1f} KB  {file['modified']}")


def analyze_examples(by_category):
    """Analyze example documentation."""
    print_header("EXAMPLES STRUCTURE")

    if 'examples' in by_category:
        examples = by_category['examples']

        # Group by type
        by_type = defaultdict(list)
        for ex in examples:
            if 'signals/' in ex['path']:
                # Extract signal type (e.g., claudecode-signals)
                parts = ex['path'].split('/')
                if len(parts) >= 3:
                    signal_type = parts[2]
                    by_type[f"signals/{signal_type}"].append(ex)
            else:
                by_type['other'].append(ex)

        for type_key in sorted(by_type.keys()):
            files = by_type[type_key]
            total_size = sum(f['size_bytes'] for f in files)
            print(f"\n  {type_key:40} {len(files):2} files  {total_size/1024:6.1f} KB")


def analyze_root_files(by_category):
    """Analyze root-level markdown files."""
    print_header("ROOT LEVEL FILES")

    if 'root' in by_category:
        for file in sorted(by_category['root'], key=lambda x: x['path']):
            size_kb = file['size_bytes'] / 1024
            print(f"  {file['path']:40} {size_kb:6.1f} KB  Modified: {file['modified']}")


def main():
    """Main analysis function."""
    data = load_inventory()

    print(f"\nMarkdown Inventory Analysis")
    print(f"Generated: {data['generated_at']}")
    print(f"Total files: {data['total_files']}")

    by_category = analyze_by_category(data)
    analyze_root_files(by_category)
    analyze_documentation_structure(by_category)
    analyze_examples(by_category)
    analyze_by_age(data)
    analyze_stale_files(data)

    print("\n")


if __name__ == '__main__':
    main()
