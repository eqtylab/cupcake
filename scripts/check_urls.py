#!/usr/bin/env python3
"""
Check URLs in markdown and HTML files for broken links.

This script recursively scans a directory for markdown and HTML files,
extracts URLs from them, and verifies that each URL resolves successfully.
Supports URL pattern replacement for testing against staging environments.
"""

import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Dict, List, Set, Tuple


def extract_urls_from_markdown(content: str) -> List[str]:
    """Extract URLs from markdown content."""
    urls = []
    
    # Markdown links: [text](url)
    markdown_links = re.findall(r'\[(?:[^\]]*)\]\(([^)]+)\)', content)
    urls.extend(markdown_links)
    
    # Reference-style links: [text]: url
    reference_links = re.findall(r'^\[(?:[^\]]+)\]:\s*(\S+)', content, re.MULTILINE)
    urls.extend(reference_links)
    
    # Plain URLs (http/https)
    plain_urls = re.findall(r'(?:https?://[^\s\)<>]+)', content)
    urls.extend(plain_urls)
    
    return urls


def extract_urls_from_html(content: str) -> List[str]:
    """Extract URLs from HTML content."""
    urls = []
    
    # href attributes
    href_urls = re.findall(r'href=["\'](https?://[^"\']+)["\']', content, re.IGNORECASE)
    urls.extend(href_urls)
    
    # src attributes
    src_urls = re.findall(r'src=["\'](https?://[^"\']+)["\']', content, re.IGNORECASE)
    urls.extend(src_urls)
    
    return urls


def apply_replacements(url: str, replacements: Dict[str, str]) -> str:
    """Apply URL pattern replacements."""
    for from_pattern, to_pattern in replacements.items():
        url = url.replace(from_pattern, to_pattern)
    return url


def check_url(url: str, timeout: int = 10) -> Tuple[bool, int, str]:
    """
    Check if a URL resolves successfully.
    
    Returns:
        Tuple of (success, status_code, error_message)
    """
    try:
        req = urllib.request.Request(
            url,
            headers={
                'User-Agent': 'Mozilla/5.0 (compatible; URLChecker/1.0)',
            }
        )
        
        with urllib.request.urlopen(req, timeout=timeout) as response:
            status = response.getcode()
            # Consider 2xx and 3xx as success
            return (200 <= status < 400, status, "")
            
    except urllib.error.HTTPError as e:
        return (False, e.code, f"HTTP {e.code}: {e.reason}")
    except urllib.error.URLError as e:
        return (False, 0, f"URL Error: {e.reason}")
    except Exception as e:
        return (False, 0, f"Error: {str(e)}")


def get_domain(url: str) -> str:
    """Extract domain from URL."""
    # Remove protocol
    if '://' in url:
        url = url.split('://', 1)[1]
    # Remove path
    domain = url.split('/')[0]
    # Remove port
    domain = domain.split(':')[0]
    return domain.lower()


def is_valid_http_url(url: str, skip_domains: Set[str] = None) -> bool:
    """Check if URL is a valid http/https URL."""
    # Filter out relative links, anchors, and non-http protocols
    if not url.startswith(('http://', 'https://')):
        return False
    
    # Filter out localhost and internal IPs
    if any(pattern in url.lower() for pattern in ['localhost', '127.0.0.1', '0.0.0.0']):
        return False
    
    # Filter out example/placeholder domains
    if any(domain in url.lower() for domain in ['example.com', 'example.org']):
        return False
    
    # Filter out skip domains
    if skip_domains:
        domain = get_domain(url)
        if any(skip in domain for skip in skip_domains):
            return False
    
    return True


def scan_directory(root_dir: Path, exclude_patterns: List[str] = None) -> List[Tuple[Path, str]]:
    """
    Recursively scan directory for markdown and HTML files.
    
    Returns:
        List of (file_path, content) tuples
    """
    if exclude_patterns is None:
        exclude_patterns = [
            'node_modules',
            '.git',
            '__pycache__',
            'target',
            'dist',
            'build',
            '.venv',
            'venv',
        ]
    
    files = []
    
    for root, dirs, filenames in os.walk(root_dir):
        # Filter out excluded directories
        dirs[:] = [d for d in dirs if d not in exclude_patterns]
        
        for filename in filenames:
            if filename.endswith(('.md', '.markdown', '.html', '.htm')):
                file_path = Path(root) / filename
                try:
                    with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                        content = f.read()
                    files.append((file_path, content))
                except Exception as e:
                    print(f"Warning: Could not read {file_path}: {e}", file=sys.stderr)
    
    return files


def main():
    parser = argparse.ArgumentParser(
        description='Check URLs in markdown and HTML files for broken links.',
        formatter_class=argparse.RawTextHelpFormatter,
    )
    
    parser.add_argument(
        'directory',
        nargs='?',
        default='.',
        help='Directory to scan (default: current directory)'
    )
    
    parser.add_argument(
        '--replacements',
        type=str,
        default=None,
        help='JSON string mapping URL patterns to replace (e.g. \'{"docs.eqty.io": "docs.staging.eqty.io"}\')'
    )
    
    parser.add_argument(
        '--timeout',
        type=int,
        default=10,
        help='Timeout for URL checks in seconds (default: 10)'
    )
    
    parser.add_argument(
        '--verbose',
        '-v',
        action='store_true',
        help='Show verbose output including successful checks'
    )
    
    parser.add_argument(
        '--workers',
        '-w',
        type=int,
        default=8,
        help='Number of parallel workers for URL checks (default: 8)'
    )
    
    parser.add_argument(
        '--skip-domains',
        type=str,
        default=None,
        help='Comma-separated list of domains to skip (e.g. "github.com,twitter.com")'
    )
    
    args = parser.parse_args()
    
    # Parse replacements
    replacements = {}
    
    # First check environment variable
    env_replacements = os.environ.get('URL_REPLACEMENTS', '').strip()
    if env_replacements:
        try:
            replacements.update(json.loads(env_replacements))
        except json.JSONDecodeError as e:
            print(f"Error parsing URL_REPLACEMENTS env var: {e}", file=sys.stderr)
            return 1
    
    # Then override with command-line argument
    if args.replacements:
        try:
            replacements.update(json.loads(args.replacements))
        except json.JSONDecodeError as e:
            print(f"Error parsing --replacements argument: {e}", file=sys.stderr)
            return 1
    
    if replacements:
        print(f"Using URL replacements: {json.dumps(replacements, indent=2)}")
        print()
    
    # Parse skip domains (env var first, then CLI arg)
    skip_domains: Set[str] = set()
    env_skip = os.environ.get('SKIP_DOMAINS', '').strip()
    if env_skip:
        skip_domains.update(d.strip().lower() for d in env_skip.split(',') if d.strip())
    if args.skip_domains:
        skip_domains.update(d.strip().lower() for d in args.skip_domains.split(',') if d.strip())
    if skip_domains:
        print(f"Skipping domains: {', '.join(sorted(skip_domains))}")
        print()
    
    # Scan directory
    root_dir = Path(args.directory).resolve()
    if not root_dir.exists():
        print(f"Error: Directory {root_dir} does not exist", file=sys.stderr)
        return 1
    
    print(f"Scanning {root_dir} for markdown and HTML files...")
    files = scan_directory(root_dir)
    print(f"Found {len(files)} files to scan")
    print()
    
    # Extract all URLs
    all_urls: Set[str] = set()
    url_sources: Dict[str, List[Path]] = {}  # Track which files contain which URLs
    
    for file_path, content in files:
        if file_path.suffix.lower() in ('.md', '.markdown'):
            urls = extract_urls_from_markdown(content)
        else:
            urls = extract_urls_from_html(content)
        
        for url in urls:
            if is_valid_http_url(url, skip_domains):
                # Apply replacements
                replaced_url = apply_replacements(url, replacements)
                all_urls.add(replaced_url)
                
                # Track source files
                if replaced_url not in url_sources:
                    url_sources[replaced_url] = []
                url_sources[replaced_url].append(file_path)
    
    print(f"Found {len(all_urls)} unique URLs to check (using {args.workers} workers)")
    print()
    
    # Check URLs in parallel
    broken_links: List[Tuple[str, int, str, List[Path]]] = []
    checked = 0
    total = len(all_urls)
    urls_list = sorted(all_urls)
    
    def check_single_url(url: str) -> Tuple[str, bool, int, str]:
        success, status_code, error_msg = check_url(url, timeout=args.timeout)
        return (url, success, status_code, error_msg)
    
    with ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = {executor.submit(check_single_url, url): url for url in urls_list}
        
        for future in as_completed(futures):
            checked += 1
            url, success, status_code, error_msg = future.result()
            
            if not args.verbose:
                print(f"Checking URLs... {checked}/{total}", end='\r')
            
            if not success:
                broken_links.append((url, status_code, error_msg, url_sources[url]))
                if args.verbose:
                    print(f"✗ {url}: {error_msg}")
            elif args.verbose:
                print(f"✓ {url}: {status_code}")
    
    print()  # Clear progress line
    print()
    
    # Report results
    if broken_links:
        print("=" * 80)
        print(f"BROKEN LINKS FOUND: {len(broken_links)}")
        print("=" * 80)
        print()
        
        for url, status_code, error_msg, source_files in broken_links:
            print(f"✗ {url}")
            print(f"  Status: {error_msg}")
            print(f"  Found in:")
            for source_file in source_files[:5]:  # Limit to first 5 files
                rel_path = source_file.relative_to(root_dir)
                print(f"    - {rel_path}")
            if len(source_files) > 5:
                print(f"    ... and {len(source_files) - 5} more files")
            print()
        
        return 1
    else:
        print("=" * 80)
        print("✓ ALL LINKS OK")
        print("=" * 80)
        print(f"Checked {len(all_urls)} unique URLs across {len(files)} files")
        return 0


if __name__ == '__main__':
    sys.exit(main())

