# METADATA
# scope: package
# description: Helper functions for secure path validation and normalization
package cupcake.helpers.paths

import rego.v1

# Check if file path targets a protected directory
# Handles case-insensitive matching and path obfuscation
# Prevents bypass via: .//./protected/file, Protected/File, etc.
targets_protected(file_path, protected_path) if {
	lower_path := lower(file_path)
	lower_protected := lower(protected_path)
	contains(lower_path, lower_protected)
}

targets_protected(file_path, protected_path) if {
	normalized := normalize(file_path)
	lower_normalized := lower(normalized)
	lower_protected := lower(protected_path)
	contains(lower_normalized, lower_protected)
}

# Normalize path by removing redundant separators and relative references
# Prevents bypass via: ./file, .//file, ././file
normalize(file_path) := normalized if {
	# Remove multiple consecutive slashes: // -> /
	step1 := regex.replace(file_path, `/{2,}`, "/")

	# Remove /./  sequences
	step2 := regex.replace(step1, `/\./`, "/")

	# Remove leading ./
	normalized := regex.replace(step2, `^\./`, "")
}

# Check if path is absolute (Unix or Windows)
# Unix: /path/to/file
# Windows: C:\path\to\file or C:/path/to/file
is_absolute(file_path) if {
	# Unix absolute path
	startswith(file_path, "/")
}

is_absolute(file_path) if {
	# Windows absolute path: C:\ or C:/
	regex.match(`^[A-Za-z]:[/\\]`, file_path)
}

# Check if path attempts to escape parent directory
# Detects: ../../../etc/passwd
escapes_directory(file_path) if {
	contains(file_path, "../")
}

# Get the directory component of a path
# /path/to/file.txt -> /path/to
get_directory(file_path) := dir if {
	# Split on / or \
	parts := regex.split(`[/\\]`, file_path)

	# Get all parts except last
	count(parts) > 1
	dir_parts := array.slice(parts, 0, count(parts) - 1)

	# Join back together
	dir := concat("/", dir_parts)
}

# Get the filename component of a path
# /path/to/file.txt -> file.txt
get_filename(file_path) := filename if {
	# Split on / or \
	parts := regex.split(`[/\\]`, file_path)

	# Get last part
	count(parts) > 0
	filename := parts[count(parts) - 1]
}

# Get the file extension
# file.txt -> txt
# archive.tar.gz -> gz
get_extension(file_path) := ext if {
	filename := get_filename(file_path)
	contains(filename, ".")

	# Split on .
	parts := split(filename, ".")

	# Get last part
	count(parts) > 1
	ext := parts[count(parts) - 1]
}

# Check if path has a specific extension
has_extension(file_path, extension) if {
	ext := get_extension(file_path)
	lower(ext) == lower(extension)
}
