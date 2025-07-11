Title: directories - Rust

URL Source: https://docs.rs/directories/latest/directories/

Markdown Content:
Expand description

The _directories_ crate is

- a tiny library with a minimal API (3 structs, 4 factory functions, getters)
- that provides the platform-specific, user-accessible locations
- for finding and storing configuration, cache and other data
- on Linux, Redox, Windows (≥ Vista) and macOS.

The library provides the location of these directories by leveraging the mechanisms defined by

- the [XDG base directory](https://standards.freedesktop.org/basedir-spec/basedir-spec-latest.html) and the [XDG user directory](https://www.freedesktop.org/wiki/Software/xdg-user-dirs/) specifications on Linux,
- the [Known Folder](<https://msdn.microsoft.com/en-us/library/windows/desktop/bb776911(v=vs.85).aspx>) system on Windows, and
- the [Standard Directories](https://developer.apple.com/library/content/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemOverview/FileSystemOverview.html#//apple_ref/doc/uid/TP40010672-CH2-SW6) on macOS.

[Base Dirs](https://docs.rs/directories/latest/directories/struct.BaseDirs.html "struct directories::BaseDirs")`BaseDirs` provides paths of user-invisible standard directories, following the conventions of the operating system the library is running on.[Project Dirs](https://docs.rs/directories/latest/directories/struct.ProjectDirs.html "struct directories::ProjectDirs")`ProjectDirs` computes the location of cache, config or data directories for a specific application, which are derived from the standard directories and the name of the project/organization.[User Dirs](https://docs.rs/directories/latest/directories/struct.UserDirs.html "struct directories::UserDirs")`UserDirs` provides paths of user-facing standard directories, following the conventions of the operating system the library is running on.
