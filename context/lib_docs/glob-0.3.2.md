Title: glob - Rust

URL Source: https://docs.rs/glob/latest/glob/

Markdown Content:
Expand description

Support for matching file paths against Unix shell style patterns.

The `glob` and `glob_with` functions allow querying the filesystem for all files that match a particular pattern (similar to the libc `glob` function). The methods on the `Pattern` type provide functionality for checking if individual paths match a particular pattern (similar to the libc `fnmatch` function).

For consistency across platforms, and for Windows support, this module is implemented entirely in Rust rather than deferring to the libc `glob`/`fnmatch` functions.

[§](https://docs.rs/glob/latest/glob/#examples)Examples
-------------------------------------------------------

To print all jpg files in `/media/` and all of its subdirectories.

```
use glob::glob;

for entry in glob("/media/**/*.jpg").expect("Failed to read glob pattern") {
    match entry {
        Ok(path) => println!("{:?}", path.display()),
        Err(e) => println!("{:?}", e),
    }
}
```

To print all files containing the letter “a”, case insensitive, in a `local` directory relative to the current working directory. This ignores errors instead of printing them.

```
use glob::glob_with;
use glob::MatchOptions;

let options = MatchOptions {
    case_sensitive: false,
    require_literal_separator: false,
    require_literal_leading_dot: false,
};
for entry in glob_with("local/*a*", options).unwrap() {
    if let Ok(path) = entry {
        println!("{:?}", path.display())
    }
}
```

[Glob Error](https://docs.rs/glob/latest/glob/struct.GlobError.html "struct glob::GlobError")A glob iteration error.[Match Options](https://docs.rs/glob/latest/glob/struct.MatchOptions.html "struct glob::MatchOptions")Configuration options to modify the behaviour of `Pattern::matches_with(..)`.[Paths](https://docs.rs/glob/latest/glob/struct.Paths.html "struct glob::Paths")An iterator that yields `Path`s from the filesystem that match a particular pattern.[Pattern](https://docs.rs/glob/latest/glob/struct.Pattern.html "struct glob::Pattern")A compiled Unix shell style pattern.[Pattern Error](https://docs.rs/glob/latest/glob/struct.PatternError.html "struct glob::PatternError")A pattern parsing error.[glob](https://docs.rs/glob/latest/glob/fn.glob.html "fn glob::glob")Return an iterator that produces all the `Path`s that match the given pattern using default match options, which may be absolute or relative to the current working directory.[glob_ with](https://docs.rs/glob/latest/glob/fn.glob_with.html "fn glob::glob_with")Return an iterator that produces all the `Path`s that match the given pattern using the specified match options, which may be absolute or relative to the current working directory.[Glob Result](https://docs.rs/glob/latest/glob/type.GlobResult.html "type glob::GlobResult")An alias for a glob iteration result.