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


class Config:
    """Configuration for URL checking."""
    
    def __init__(self, args: argparse.Namespace):
        self.directory = Path(args.directory).resolve()
        self.timeout = args.timeout
        self.verbose = args.verbose
        self.workers = args.workers
        self.replacements = self._parse_replacements(args)
        self.skip_domains = self._parse_skip_domains(args)
        self.skip_urls = self._parse_skip_urls(args)
        self.skip_files = self._parse_skip_files(args)
        self.markdown_patterns = self._parse_patterns(args, 'markdown_patterns', 'MARKDOWN_PATTERNS', ['.md', '.markdown'])
        self.html_patterns = self._parse_patterns(args, 'html_patterns', 'HTML_PATTERNS', ['.html', '.htm'])
        self.file_patterns = self._parse_patterns(args, 'file_patterns', 'FILE_PATTERNS', [])
    
    def _parse_patterns(self, args: argparse.Namespace, arg_name: str, env_name: str, default: List[str]) -> List[str]:
        """Parse file patterns from env var and CLI args."""
        patterns = list(default)
        
        env_patterns = os.environ.get(env_name, '').strip()
        if env_patterns:
            patterns = [p.strip() for p in env_patterns.split(',') if p.strip()]
        
        arg_value = getattr(args, arg_name, None)
        if arg_value:
            patterns = [p.strip() for p in arg_value.split(',') if p.strip()]
        
        return patterns
    
    def _parse_replacements(self, args: argparse.Namespace) -> Dict[str, str]:
        """Parse URL replacements from env var and CLI args."""
        replacements = {}
        
        env_replacements = os.environ.get('URL_REPLACEMENTS', '').strip()
        if env_replacements:
            try:
                replacements.update(json.loads(env_replacements))
            except json.JSONDecodeError as e:
                print(f"Error parsing URL_REPLACEMENTS env var: {e}", file=sys.stderr)
                sys.exit(1)
        
        if args.replacements:
            try:
                replacements.update(json.loads(args.replacements))
            except json.JSONDecodeError as e:
                print(f"Error parsing --replacements argument: {e}", file=sys.stderr)
                sys.exit(1)
        
        return replacements
    
    def _parse_skip_domains(self, args: argparse.Namespace) -> Set[str]:
        """Parse skip domains from env var and CLI args."""
        skip_domains = set()
        
        env_skip = os.environ.get('SKIP_DOMAINS', '').strip()
        if env_skip:
            skip_domains.update(d.strip().lower() for d in env_skip.split(',') if d.strip())
        
        if args.skip_domains:
            skip_domains.update(d.strip().lower() for d in args.skip_domains.split(',') if d.strip())
        
        return skip_domains
    
    def _parse_skip_urls(self, args: argparse.Namespace) -> Set[str]:
        """Parse skip URLs from env var and CLI args."""
        skip_urls = set()
        
        env_skip = os.environ.get('SKIP_URLS', '').strip()
        if env_skip:
            skip_urls.update(u.strip() for u in env_skip.split(',') if u.strip())
        
        if args.skip_urls:
            skip_urls.update(u.strip() for u in args.skip_urls.split(',') if u.strip())
        
        return skip_urls
    
    def _parse_skip_files(self, args: argparse.Namespace) -> Set[str]:
        """Parse skip files from env var and CLI args."""
        skip_files = set()
        
        env_skip = os.environ.get('SKIP_FILES', '').strip()
        if env_skip:
            skip_files.update(f.strip() for f in env_skip.split(',') if f.strip())
        
        if args.skip_files:
            skip_files.update(f.strip() for f in args.skip_files.split(',') if f.strip())
        
        return skip_files
    
    def print_config(self):
        """Print configuration summary."""
        if self.replacements:
            print(f"Using URL replacements: {json.dumps(self.replacements, indent=2)}")
            print()
        
        if self.skip_domains:
            print(f"Skipping domains: {', '.join(sorted(self.skip_domains))}")
            print()
        
        if self.skip_urls:
            print(f"Skipping URLs: {len(self.skip_urls)} URL(s)")
            print()
        
        if self.skip_files:
            print(f"Skipping files: {', '.join(sorted(self.skip_files))}")
            print()
        
        print(f"Markdown patterns: {', '.join(self.markdown_patterns)}")
        print(f"HTML patterns: {', '.join(self.html_patterns)}")
        if self.file_patterns:
            print(f"Additional file patterns: {', '.join(self.file_patterns)}")
        print()


class URLExtractor:
    """Extract URLs from markdown and HTML files."""
    
    EXCLUDE_PATTERNS = [
        'node_modules', '.git', '__pycache__', 'target',
        'dist', 'build', '.venv', 'venv',
    ]
    
    @staticmethod
    def extract_from_markdown(content: str) -> List[str]:
        """Extract URLs from markdown content."""
        urls = []
        
        # Markdown links: [text](url)
        markdown_links = re.findall(r'\[(?:[^\]]*)\]\(([^)]+)\)', content)
        urls.extend(markdown_links)
        
        # Reference-style links: [text]: url
        reference_links = re.findall(r'^\[(?:[^\]]+)\]:\s*(\S+)', content, re.MULTILINE)
        urls.extend(reference_links)
        
        # Plain URLs (http/https) - exclude common trailing punctuation/delimiters
        plain_urls = re.findall(r'(?:https?://[^\s\)<>\[\]{}"\',;]+)', content)
        urls.extend(plain_urls)
        
        return urls
    
    @staticmethod
    def extract_from_html(content: str) -> List[str]:
        """Extract URLs from HTML content."""
        urls = []
        
        # href attributes
        href_urls = re.findall(r'href=["\'](https?://[^"\']+)["\']', content, re.IGNORECASE)
        urls.extend(href_urls)
        
        # src attributes
        src_urls = re.findall(r'src=["\'](https?://[^"\']+)["\']', content, re.IGNORECASE)
        urls.extend(src_urls)
        
        return urls
    
    @classmethod
    def scan_directory(cls, root_dir: Path, skip_files: Set[str], markdown_patterns: List[str], html_patterns: List[str], file_patterns: List[str]) -> List[Tuple[Path, str, str]]:
        """Recursively scan directory for markdown and HTML files.
        
        Returns:
            List of (file_path, content, file_type) tuples where file_type is 'markdown', 'html', or 'file'
        """
        files = []
        all_patterns = markdown_patterns + html_patterns + file_patterns
        
        for root, dirs, filenames in os.walk(root_dir):
            # Filter out excluded directories
            dirs[:] = [d for d in dirs if d not in cls.EXCLUDE_PATTERNS]
            
            for filename in filenames:
                # Check if file matches any pattern
                matches = False
                file_type = None
                
                for pattern in markdown_patterns:
                    if filename.endswith(pattern):
                        matches = True
                        file_type = 'markdown'
                        break
                
                if not matches:
                    for pattern in html_patterns:
                        if filename.endswith(pattern):
                            matches = True
                            file_type = 'html'
                            break
                
                if not matches:
                    for pattern in file_patterns:
                        if filename.endswith(pattern):
                            matches = True
                            file_type = 'file'
                            break
                
                if matches:
                    file_path = Path(root) / filename
                    
                    # Check if file should be skipped
                    if skip_files and filename in skip_files:
                        continue
                    
                    try:
                        with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                            content = f.read()
                        files.append((file_path, content, file_type))
                    except Exception as e:
                        print(f"Warning: Could not read {file_path}: {e}", file=sys.stderr)
        
        return files


class URLChecker:
    """Check URLs for validity and accessibility."""
    
    def __init__(self, config: Config):
        self.config = config
    
    @staticmethod
    def get_domain(url: str) -> str:
        """Extract domain from URL."""
        if '://' in url:
            url = url.split('://', 1)[1]
        domain = url.split('/')[0]
        domain = domain.split(':')[0]
        return domain.lower()
    
    def is_valid_url(self, url: str) -> bool:
        """Check if URL should be checked."""
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
        if self.config.skip_domains:
            domain = self.get_domain(url)
            if any(skip in domain for skip in self.config.skip_domains):
                return False
        
        # Filter out skip URLs (exact match or substring)
        if self.config.skip_urls:
            if any(skip_url in url for skip_url in self.config.skip_urls):
                return False
        
        return True
    
    def apply_replacements(self, url: str) -> str:
        """Apply URL pattern replacements."""
        for from_pattern, to_pattern in self.config.replacements.items():
            url = url.replace(from_pattern, to_pattern)
        return url
    
    def check_url(self, url: str) -> Tuple[bool, int, str]:
        """Check if a URL resolves successfully."""
        try:
            req = urllib.request.Request(
                url,
                headers={'User-Agent': 'Mozilla/5.0 (compatible; URLChecker/1.0)'}
            )
            
            with urllib.request.urlopen(req, timeout=self.config.timeout) as response:
                status = response.getcode()
                return (200 <= status < 400, status, "")
        
        except urllib.error.HTTPError as e:
            return (False, e.code, f"HTTP {e.code}: {e.reason}")
        except urllib.error.URLError as e:
            return (False, 0, f"URL Error: {e.reason}")
        except Exception as e:
            return (False, 0, f"Error: {str(e)}")


class URLCheckRunner:
    """Main runner for URL checking."""
    
    def __init__(self, config: Config):
        self.config = config
        self.checker = URLChecker(config)
        self.extractor = URLExtractor()
    
    def extract_all_urls(self, files: List[Tuple[Path, str, str]]) -> Tuple[Set[str], Dict[str, List[Path]]]:
        """Extract all URLs from files."""
        all_urls: Set[str] = set()
        url_sources: Dict[str, List[Path]] = {}
        
        for file_path, content, file_type in files:
            if file_type == 'markdown':
                urls = self.extractor.extract_from_markdown(content)
            elif file_type == 'html':
                urls = self.extractor.extract_from_html(content)
            else:
                # For generic files, try both extractors
                urls = self.extractor.extract_from_markdown(content)
                urls.extend(self.extractor.extract_from_html(content))
            
            for url in urls:
                if self.checker.is_valid_url(url):
                    replaced_url = self.checker.apply_replacements(url)
                    all_urls.add(replaced_url)
                    
                    if replaced_url not in url_sources:
                        url_sources[replaced_url] = []
                    url_sources[replaced_url].append(file_path)
        
        return all_urls, url_sources
    
    def check_urls_parallel(self, urls: Set[str], url_sources: Dict[str, List[Path]]) -> List[Tuple[str, int, str, List[Path]]]:
        """Check URLs in parallel using thread pool."""
        broken_links = []
        checked = 0
        total = len(urls)
        urls_list = sorted(urls)
        
        def check_single_url(url: str) -> Tuple[str, bool, int, str]:
            success, status_code, error_msg = self.checker.check_url(url)
            return (url, success, status_code, error_msg)
        
        with ThreadPoolExecutor(max_workers=self.config.workers) as executor:
            futures = {executor.submit(check_single_url, url): url for url in urls_list}
            
            for future in as_completed(futures):
                checked += 1
                url, success, status_code, error_msg = future.result()
                
                if not self.config.verbose:
                    print(f"Checking URLs... {checked}/{total}", end='\r')
                
                if not success:
                    broken_links.append((url, status_code, error_msg, url_sources[url]))
                    if self.config.verbose:
                        print(f"✗ {url}: {error_msg}")
                elif self.config.verbose:
                    print(f"✓ {url}: {status_code}")
        
        return broken_links
    
    def print_results(self, broken_links: List[Tuple[str, int, str, List[Path]]], total_urls: int, total_files: int):
        """Print final results."""
        print()
        print()
        
        if broken_links:
            print("=" * 80)
            print(f"BROKEN LINKS FOUND: {len(broken_links)}")
            print("=" * 80)
            print()
            
            for url, status_code, error_msg, source_files in broken_links:
                print(f"✗ {url}")
                print(f"  Status: {error_msg}")
                print(f"  Found in:")
                for source_file in source_files[:5]:
                    rel_path = source_file.relative_to(self.config.directory)
                    print(f"    - {rel_path}")
                if len(source_files) > 5:
                    print(f"    ... and {len(source_files) - 5} more files")
                print()
            
            return 1
        else:
            print("=" * 80)
            print("✓ ALL LINKS OK")
            print("=" * 80)
            print(f"Checked {total_urls} unique URLs across {total_files} files")
            return 0
    
    def run(self) -> int:
        """Run the URL checker."""
        if not self.config.directory.exists():
            print(f"Error: Directory {self.config.directory} does not exist", file=sys.stderr)
            return 1
        
        self.config.print_config()
        
        print(f"Scanning {self.config.directory} for files...")
        files = self.extractor.scan_directory(
            self.config.directory,
            self.config.skip_files,
            self.config.markdown_patterns,
            self.config.html_patterns,
            self.config.file_patterns
        )
        print(f"Found {len(files)} files to scan")
        print()
        
        all_urls, url_sources = self.extract_all_urls(files)
        print(f"Found {len(all_urls)} unique URLs to check (using {self.config.workers} workers)")
        print()
        
        broken_links = self.check_urls_parallel(all_urls, url_sources)
        
        return self.print_results(broken_links, len(all_urls), len(files))


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
    
    parser.add_argument(
        '--skip-urls',
        type=str,
        default=None,
        help='Comma-separated list of URL patterns to skip (e.g. "https://example.com/page")'
    )
    
    parser.add_argument(
        '--skip-files',
        type=str,
        default=None,
        help='Comma-separated list of filenames to skip (e.g. "README.md,CHANGELOG.md")'
    )
    
    parser.add_argument(
        '--markdown-patterns',
        type=str,
        default=None,
        help='Comma-separated list of markdown file extensions (default: ".md,.markdown")'
    )
    
    parser.add_argument(
        '--html-patterns',
        type=str,
        default=None,
        help='Comma-separated list of HTML file extensions (default: ".html,.htm")'
    )
    
    parser.add_argument(
        '--file-patterns',
        type=str,
        default=None,
        help='Comma-separated list of additional file extensions to check'
    )
    
    args = parser.parse_args()
    
    config = Config(args)
    runner = URLCheckRunner(config)
    
    return runner.run()


if __name__ == '__main__':
    sys.exit(main())
