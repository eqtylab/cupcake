# METADATA
# scope: package
# description: Tests for helper library functions
package cupcake.helpers.test

import rego.v1

import data.cupcake.helpers.commands
import data.cupcake.helpers.paths

# Test: has_verb detects command with normal spacing
test_has_verb_normal if {
	commands.has_verb("git commit", "git")
	commands.has_verb("git commit", "commit")
}

# Test: has_verb detects command with extra spaces
test_has_verb_extra_spaces if {
	commands.has_verb("git  commit", "git")
	commands.has_verb("git  commit", "commit")
	commands.has_verb("  git commit", "git")
}

# Test: has_verb with tabs
test_has_verb_tabs if {
	commands.has_verb("git\tcommit", "git")
}

# Test: has_dangerous_verb with set
test_has_dangerous_verb if {
	dangerous := {"rm", "mv", "dd"}
	commands.has_dangerous_verb("rm -rf /", dangerous)
	commands.has_dangerous_verb("mv file1 file2", dangerous)
	not commands.has_dangerous_verb("cat file", dangerous)
}

# Test: creates_symlink detection
test_creates_symlink if {
	commands.creates_symlink("ln -s source target")
	commands.creates_symlink("ln -sf source target")
	not commands.creates_symlink("ln source target") # hard link
}

# Test: symlink_involves_path
test_symlink_involves_path if {
	commands.symlink_involves_path("ln -s .cupcake foo", ".cupcake")
	commands.symlink_involves_path("ln -s foo .cupcake", ".cupcake")
	not commands.symlink_involves_path("ln -s foo bar", ".cupcake")
}

# Test: has_output_redirect
test_has_output_redirect if {
	commands.has_output_redirect("echo test > file")
	commands.has_output_redirect("echo test >> file")
	commands.has_output_redirect("cat file | grep pattern")
	commands.has_output_redirect("echo test | tee file")
}

# Test: path normalization
test_normalize_path if {
	paths.normalize("./file") == "file"
	paths.normalize(".//file") == "file"
	paths.normalize("path/./to/file") == "path/to/file"
	paths.normalize("path//to///file") == "path/to/file"
}

# Test: targets_protected with case insensitivity
test_targets_protected if {
	paths.targets_protected(".cupcake/file", ".cupcake")
	paths.targets_protected(".Cupcake/file", ".cupcake")
	paths.targets_protected(".CUPCAKE/file", ".cupcake")
}

# Test: targets_protected with path obfuscation
test_targets_protected_obfuscation if {
	paths.targets_protected("./.cupcake/file", ".cupcake")
	paths.targets_protected(".//./.cupcake/file", ".cupcake")
}

# Test: is_absolute detection
test_is_absolute if {
	paths.is_absolute("/absolute/path")
	paths.is_absolute("C:/windows/path")
	paths.is_absolute("C:\\windows\\path")
	not paths.is_absolute("relative/path")
	not paths.is_absolute("./relative/path")
}

# Test: escapes_directory detection
test_escapes_directory if {
	paths.escapes_directory("../../../etc/passwd")
	paths.escapes_directory("path/../other/file")
	not paths.escapes_directory("/absolute/path")
}

# Test: get_filename
test_get_filename if {
	paths.get_filename("/path/to/file.txt") == "file.txt"
	paths.get_filename("file.txt") == "file.txt"
	paths.get_filename("C:\\path\\to\\file.txt") == "file.txt"
}

# Test: get_extension
test_get_extension if {
	paths.get_extension("file.txt") == "txt"
	paths.get_extension("archive.tar.gz") == "gz"
	paths.get_extension("/path/to/file.txt") == "txt"
}

# Test: has_extension
test_has_extension if {
	paths.has_extension("file.txt", "txt")
	paths.has_extension("file.TXT", "txt") # case insensitive
	paths.has_extension("archive.tar.gz", "gz")
	not paths.has_extension("file.txt", "md")
}
