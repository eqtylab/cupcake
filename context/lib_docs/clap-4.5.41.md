Title: clap - Rust

URL Source: https://docs.rs/clap/latest/clap/

Markdown Content:
Expand description

> **Command Line Argument Parser for Rust**

Quick Links:

- Derive [tutorial](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html "mod clap::_derive::_tutorial") and [reference](https://docs.rs/clap/latest/clap/_derive/index.html "mod clap::_derive")
- Builder [tutorial](https://docs.rs/clap/latest/clap/_tutorial/index.html "mod clap::_tutorial") and [reference](https://docs.rs/clap/latest/clap/struct.Command.html "struct clap::Command")
- [Cookbook](https://docs.rs/clap/latest/clap/_cookbook/index.html "mod clap::_cookbook")
- [FAQ](https://docs.rs/clap/latest/clap/_faq/index.html "mod clap::_faq")
- [Discussions](https://github.com/clap-rs/clap/discussions)
- [CHANGELOG](https://github.com/clap-rs/clap/blob/v4.5.41/CHANGELOG.md) (includes major version migration guides)

### [§](https://docs.rs/clap/latest/clap/#aspirations)Aspirations

- Out of the box, users get a polished CLI experience

  - Including common argument behavior, help generation, suggested fixes for users, colored output, [shell completions](https://github.com/clap-rs/clap/tree/master/clap_complete), etc

- Flexible enough to port your existing CLI interface

  - However, we won’t necessarily streamline support for each use case

- Reasonable parse performance
- Resilient maintainership, including

  - Willing to break compatibility rather than batching up breaking changes in large releases
  - Leverage feature flags to keep to one active branch
  - Being under [WG-CLI](https://github.com/rust-cli/team/) to increase the bus factor

- We follow semver and will wait about 6-9 months between major breaking changes
- We will support the last two minor Rust releases (MSRV, currently 1.74)

While these aspirations can be at odds with fast build times and low binary size, we will still strive to keep these reasonable for the flexibility you get. Check out the [argparse-benchmarks](https://github.com/rust-cli/argparse-benchmarks-rs) for CLI parsers optimized for other use cases.

### [§](https://docs.rs/clap/latest/clap/#example)Example

Run

`$ cargo add clap --features derive`

_(See also [feature flag reference](https://docs.rs/clap/latest/clap/_features/index.html "mod clap::_features"))_

Then define your CLI in `main.rs`:

```
use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    name: String,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

fn main() {
    let args = Args::parse();

    for _ in 0..args.count {
        println!("Hello {}!", args.name);
    }
}
```

And try it out:

```
$ demo --help
A simple to use, efficient, and full-featured Command Line Argument Parser

Usage: demo[EXE] [OPTIONS] --name <NAME>

Options:
  -n, --name <NAME>    Name of the person to greet
  -c, --count <COUNT>  Number of times to greet [default: 1]
  -h, --help           Print help
  -V, --version        Print version

$ demo --name Me
Hello Me!
```

_(version number and `.exe` extension on windows replaced by placeholders)_

See also the derive [tutorial](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html "mod clap::_derive::_tutorial") and [reference](https://docs.rs/clap/latest/clap/_derive/index.html "mod clap::_derive")

Augment clap:

- [wild](https://crates.io/crates/wild) for supporting wildcards (`*`) on Windows like you do Linux
- [argfile](https://crates.io/crates/argfile) for loading additional arguments from a file (aka response files)
- [shadow-rs](https://crates.io/crates/shadow-rs) for generating `Command::long_version`
- [clap_mangen](https://crates.io/crates/clap_mangen) for generating man page source (roff)
- [clap_complete](https://crates.io/crates/clap_complete) for shell completion support

CLI Helpers

- [clio](https://crates.io/crates/clio) for reading/writing to files specified as arguments
- [clap-verbosity-flag](https://crates.io/crates/clap-verbosity-flag)
- [clap-cargo](https://crates.io/crates/clap-cargo)
- [colorchoice-clap](https://crates.io/crates/colorchoice-clap)

Testing

- [`trycmd`](https://crates.io/crates/trycmd): Bulk snapshot testing
- [`snapbox`](https://crates.io/crates/snapbox): Specialized snapshot testing
- [`assert_cmd`](https://crates.io/crates/assert_cmd) and [`assert_fs`](https://crates.io/crates/assert_fs): Customized testing

Documentation:

- [Command-line Apps for Rust](https://rust-cli.github.io/book/index.html) book

[\_cookbook](https://docs.rs/clap/latest/clap/_cookbook/index.html "mod clap::_cookbook")`unstable-doc`Documentation: Cookbook[\_derive](https://docs.rs/clap/latest/clap/_derive/index.html "mod clap::_derive")`unstable-doc`Documentation: Derive Reference[\_faq](https://docs.rs/clap/latest/clap/_faq/index.html "mod clap::_faq")`unstable-doc`Documentation: FAQ[\_features](https://docs.rs/clap/latest/clap/_features/index.html "mod clap::_features")`unstable-doc`Documentation: Feature Flags[\_tutorial](https://docs.rs/clap/latest/clap/_tutorial/index.html "mod clap::_tutorial")`unstable-doc`Tutorial for the Builder API[builder](https://docs.rs/clap/latest/clap/builder/index.html "mod clap::builder")Define [`Command`](https://docs.rs/clap/latest/clap/struct.Command.html "struct clap::Command") line [arguments](https://docs.rs/clap/latest/clap/struct.Arg.html "struct clap::Arg")[error](https://docs.rs/clap/latest/clap/error/index.html "mod clap::error")Error reporting[parser](https://docs.rs/clap/latest/clap/parser/index.html "mod clap::parser")[`Command`](https://docs.rs/clap/latest/clap/struct.Command.html "struct clap::Command") line argument parser[arg](https://docs.rs/clap/latest/clap/macro.arg.html "macro clap::arg")Create an [`Arg`](https://docs.rs/clap/latest/clap/struct.Arg.html "struct clap::Arg") from a usage string.[command](https://docs.rs/clap/latest/clap/macro.command.html "macro clap::command")Allows you to build the `Command` instance from your Cargo.toml at compile time.[crate\_ authors](https://docs.rs/clap/latest/clap/macro.crate_authors.html "macro clap::crate_authors")Allows you to pull the authors for the command from your Cargo.toml at compile time in the form: `"author1 lastname <author1@example.com>:author2 lastname <author2@example.com>"`[crate\_ description](https://docs.rs/clap/latest/clap/macro.crate_description.html "macro clap::crate_description")Allows you to pull the description from your Cargo.toml at compile time.[crate\_ name](https://docs.rs/clap/latest/clap/macro.crate_name.html "macro clap::crate_name")Allows you to pull the name from your Cargo.toml at compile time.[crate\_ version](https://docs.rs/clap/latest/clap/macro.crate_version.html "macro clap::crate_version")Allows you to pull the version from your Cargo.toml at compile time as `MAJOR.MINOR.PATCH_PKGVERSION_PRE`[value\_ parser](https://docs.rs/clap/latest/clap/macro.value_parser.html "macro clap::value_parser")Select a [`ValueParser`](https://docs.rs/clap/latest/clap/builder/struct.ValueParser.html "struct clap::builder::ValueParser") implementation from the intended type[Arg](https://docs.rs/clap/latest/clap/struct.Arg.html "struct clap::Arg")The abstract representation of a command line argument. Used to set all the options and relationships that define a valid argument for the program.[ArgGroup](https://docs.rs/clap/latest/clap/struct.ArgGroup.html "struct clap::ArgGroup")Family of related [arguments](https://docs.rs/clap/latest/clap/struct.Arg.html "struct clap::Arg").[ArgMatches](https://docs.rs/clap/latest/clap/struct.ArgMatches.html "struct clap::ArgMatches")Container for parse results.[Command](https://docs.rs/clap/latest/clap/struct.Command.html "struct clap::Command")Build a command-line interface.[Id](https://docs.rs/clap/latest/clap/struct.Id.html "struct clap::Id")[`Arg`](https://docs.rs/clap/latest/clap/struct.Arg.html "struct clap::Arg") or [`ArgGroup`](https://docs.rs/clap/latest/clap/struct.ArgGroup.html "struct clap::ArgGroup") identifier[ArgAction](https://docs.rs/clap/latest/clap/enum.ArgAction.html "enum clap::ArgAction")Behavior of arguments when they are encountered while parsing[Color Choice](https://docs.rs/clap/latest/clap/enum.ColorChoice.html "enum clap::ColorChoice")Represents the color preferences for program output[Value Hint](https://docs.rs/clap/latest/clap/enum.ValueHint.html "enum clap::ValueHint")Provide shell with hint on how to complete an argument.[Args](https://docs.rs/clap/latest/clap/trait.Args.html "trait clap::Args")Parse a set of arguments into a user-defined container.[Command Factory](https://docs.rs/clap/latest/clap/trait.CommandFactory.html "trait clap::CommandFactory")Create a [`Command`](https://docs.rs/clap/latest/clap/struct.Command.html "struct clap::Command") relevant for a user-defined container.[From ArgMatches](https://docs.rs/clap/latest/clap/trait.FromArgMatches.html "trait clap::FromArgMatches")Converts an instance of [`ArgMatches`](https://docs.rs/clap/latest/clap/struct.ArgMatches.html "struct clap::ArgMatches") to a user-defined container.[Parser](https://docs.rs/clap/latest/clap/trait.Parser.html "trait clap::Parser")Parse command-line arguments into `Self`.[Subcommand](https://docs.rs/clap/latest/clap/trait.Subcommand.html "trait clap::Subcommand")Parse a sub-command into a user-defined enum.[Value Enum](https://docs.rs/clap/latest/clap/trait.ValueEnum.html "trait clap::ValueEnum")Parse arguments into enums.[Error](https://docs.rs/clap/latest/clap/type.Error.html "type clap::Error")Command Line Argument Parser Error
