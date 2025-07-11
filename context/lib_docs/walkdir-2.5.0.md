Title: walkdir - Rust

URL Source: https://docs.rs/walkdir/latest/walkdir/

Markdown Content:
Expand description

Crate `walkdir` provides an efficient and cross platform implementation of recursive directory traversal. Several options are exposed to control iteration, such as whether to follow symbolic links (default off), limit the maximum number of simultaneous open file descriptors and the ability to efficiently skip descending into directories.

To use this crate, add `walkdir` as a dependency to your project’s `Cargo.toml`:

```
[dependencies]
walkdir = "2"
```

## [§](https://docs.rs/walkdir/latest/walkdir/#from-the-top)From the top

The [`WalkDir`](https://docs.rs/walkdir/latest/walkdir/struct.WalkDir.html) type builds iterators. The [`DirEntry`](https://docs.rs/walkdir/latest/walkdir/struct.DirEntry.html) type describes values yielded by the iterator. Finally, the [`Error`](https://docs.rs/walkdir/latest/walkdir/struct.Error.html) type is a small wrapper around [`std::io::Error`](https://doc.rust-lang.org/stable/std/io/struct.Error.html) with additional information, such as if a loop was detected while following symbolic links (not enabled by default).

## [§](https://docs.rs/walkdir/latest/walkdir/#example)Example

The following code recursively iterates over the directory given and prints the path for each entry:

```
use walkdir::WalkDir;

for entry in WalkDir::new("foo") {
    println!("{}", entry?.path().display());
}
```

Or, if you’d like to iterate over all entries and ignore any errors that may arise, use [`filter_map`](https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#method.filter_map). (e.g., This code below will silently skip directories that the owner of the running process does not have permission to access.)

```
use walkdir::WalkDir;

for entry in WalkDir::new("foo").into_iter().filter_map(|e| e.ok()) {
    println!("{}", entry.path().display());
}
```

## [§](https://docs.rs/walkdir/latest/walkdir/#example-follow-symbolic-links)Example: follow symbolic links

The same code as above, except [`follow_links`](https://docs.rs/walkdir/latest/walkdir/struct.WalkDir.html#method.follow_links) is enabled:

```
use walkdir::WalkDir;

for entry in WalkDir::new("foo").follow_links(true) {
    println!("{}", entry?.path().display());
}
```

This uses the [`filter_entry`](https://docs.rs/walkdir/latest/walkdir/struct.IntoIter.html#method.filter_entry) iterator adapter to avoid yielding hidden files and directories efficiently (i.e. without recursing into hidden directories):

```
use walkdir::{DirEntry, WalkDir};

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with("."))
         .unwrap_or(false)
}

let walker = WalkDir::new("foo").into_iter();
for entry in walker.filter_entry(|e| !is_hidden(e)) {
    println!("{}", entry?.path().display());
}
```

[DirEntry](https://docs.rs/walkdir/latest/walkdir/struct.DirEntry.html "struct walkdir::DirEntry")A directory entry.[Error](https://docs.rs/walkdir/latest/walkdir/struct.Error.html "struct walkdir::Error")An error produced by recursively walking a directory.[Filter Entry](https://docs.rs/walkdir/latest/walkdir/struct.FilterEntry.html "struct walkdir::FilterEntry")A recursive directory iterator that skips entries.[Into Iter](https://docs.rs/walkdir/latest/walkdir/struct.IntoIter.html "struct walkdir::IntoIter")An iterator for recursively descending into a directory.[WalkDir](https://docs.rs/walkdir/latest/walkdir/struct.WalkDir.html "struct walkdir::WalkDir")A builder to create an iterator for recursively walking a directory.[DirEntry Ext](https://docs.rs/walkdir/latest/walkdir/trait.DirEntryExt.html "trait walkdir::DirEntryExt")Unix-specific extension methods for `walkdir::DirEntry`[Result](https://docs.rs/walkdir/latest/walkdir/type.Result.html "type walkdir::Result")A result type for walkdir operations.
