Title: tokio - Rust

URL Source: https://docs.rs/tokio/latest/tokio/

Markdown Content:
Expand description

A runtime for writing reliable network applications without compromising speed.

Tokio is an event-driven, non-blocking I/O platform for writing asynchronous applications with the Rust programming language. At a high level, it provides a few major components:

- Tools for [working with asynchronous tasks](https://docs.rs/tokio/latest/tokio/#working-with-tasks), including [synchronization primitives and channels](https://docs.rs/tokio/latest/tokio/sync/index.html "mod tokio::sync") and [timeouts, sleeps, and intervals](https://docs.rs/tokio/latest/tokio/time/index.html "mod tokio::time").
- APIs for [performing asynchronous I/O](https://docs.rs/tokio/latest/tokio/#asynchronous-io), including [TCP and UDP](https://docs.rs/tokio/latest/tokio/net/index.html "mod tokio::net") sockets, [filesystem](https://docs.rs/tokio/latest/tokio/fs/index.html "mod tokio::fs") operations, and [process](https://docs.rs/tokio/latest/tokio/process/index.html "mod tokio::process") and [signal](https://docs.rs/tokio/latest/tokio/signal/index.html "mod tokio::signal") management.
- A [runtime](https://docs.rs/tokio/latest/tokio/runtime/index.html "mod tokio::runtime") for executing asynchronous code, including a task scheduler, an I/O driver backed by the operating system’s event queue (`epoll`, `kqueue`, `IOCP`, etc…), and a high performance timer.

Guide level documentation is found on the [website](https://tokio.rs/tokio/tutorial).

## [§](https://docs.rs/tokio/latest/tokio/#a-tour-of-tokio)A Tour of Tokio

Tokio consists of a number of modules that provide a range of functionality essential for implementing asynchronous applications in Rust. In this section, we will take a brief tour of Tokio, summarizing the major APIs and their uses.

The easiest way to get started is to enable all features. Do this by enabling the `full` feature flag:

`tokio = { version = "1", features = ["full"] }`

Tokio is great for writing applications and most users in this case shouldn’t worry too much about what features they should pick. If you’re unsure, we suggest going with `full` to ensure that you don’t run into any road blocks while you’re building your application.

##### [§](https://docs.rs/tokio/latest/tokio/#example)Example

This example shows the quickest way to get started with Tokio.

`tokio = { version = "1", features = ["full"] }`

#### [§](https://docs.rs/tokio/latest/tokio/#authoring-libraries)Authoring libraries

As a library author your goal should be to provide the lightest weight crate that is based on Tokio. To achieve this you should ensure that you only enable the features you need. This allows users to pick up your crate without having to enable unnecessary features.

##### [§](https://docs.rs/tokio/latest/tokio/#example-1)Example

This example shows how you may want to import features for a library that just needs to `tokio::spawn` and use a `TcpStream`.

`tokio = { version = "1", features = ["rt", "net"] }`

### [§](https://docs.rs/tokio/latest/tokio/#working-with-tasks)Working With Tasks

Asynchronous programs in Rust are based around lightweight, non-blocking units of execution called [_tasks_](https://docs.rs/tokio/latest/tokio/#working-with-tasks). The [`tokio::task`](https://docs.rs/tokio/latest/tokio/task/index.html "mod tokio::task") module provides important tools for working with tasks:

- The [`spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html "fn tokio::task::spawn") function and [`JoinHandle`](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html "struct tokio::task::JoinHandle") type, for scheduling a new task on the Tokio runtime and awaiting the output of a spawned task, respectively,
- Functions for [running blocking operations](https://docs.rs/tokio/latest/tokio/task/index.html#blocking-and-yielding) in an asynchronous task context.

The [`tokio::task`](https://docs.rs/tokio/latest/tokio/task/index.html "mod tokio::task") module is present only when the “rt” feature flag is enabled.

The [`tokio::sync`](https://docs.rs/tokio/latest/tokio/sync/index.html "mod tokio::sync") module contains synchronization primitives to use when needing to communicate or share data. These include:

- channels ([`oneshot`](https://docs.rs/tokio/latest/tokio/sync/oneshot/index.html "mod tokio::sync::oneshot"), [`mpsc`](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html "mod tokio::sync::mpsc"), [`watch`](https://docs.rs/tokio/latest/tokio/sync/watch/index.html "mod tokio::sync::watch"), and [`broadcast`](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html "mod tokio::sync::broadcast")), for sending values between tasks,
- a non-blocking [`Mutex`](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html "struct tokio::sync::Mutex"), for controlling access to a shared, mutable value,
- an asynchronous [`Barrier`](https://docs.rs/tokio/latest/tokio/sync/struct.Barrier.html "struct tokio::sync::Barrier") type, for multiple tasks to synchronize before beginning a computation.

The `tokio::sync` module is present only when the “sync” feature flag is enabled.

The [`tokio::time`](https://docs.rs/tokio/latest/tokio/time/index.html "mod tokio::time") module provides utilities for tracking time and scheduling work. This includes functions for setting [timeouts](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html "fn tokio::time::timeout") for tasks, [sleeping](https://docs.rs/tokio/latest/tokio/time/fn.sleep.html "fn tokio::time::sleep") work to run in the future, or [repeating an operation at an interval](https://docs.rs/tokio/latest/tokio/time/fn.interval.html "fn tokio::time::interval").

In order to use `tokio::time`, the “time” feature flag must be enabled.

Finally, Tokio provides a _runtime_ for executing asynchronous tasks. Most applications can use the [`#[tokio::main]`](https://docs.rs/tokio/latest/tokio/attr.main.html) macro to run their code on the Tokio runtime. However, this macro provides only basic configuration options. As an alternative, the [`tokio::runtime`](https://docs.rs/tokio/latest/tokio/runtime/index.html "mod tokio::runtime") module provides more powerful APIs for configuring and managing runtimes. You should use that module if the `#[tokio::main]` macro doesn’t provide the functionality you need.

Using the runtime requires the “rt” or “rt-multi-thread” feature flags, to enable the current-thread [single-threaded scheduler](https://docs.rs/tokio/latest/tokio/runtime/index.html#current-thread-scheduler) and the [multi-thread scheduler](https://docs.rs/tokio/latest/tokio/runtime/index.html#multi-thread-scheduler), respectively. See the [`runtime` module documentation](https://docs.rs/tokio/latest/tokio/runtime/index.html#runtime-scheduler) for details. In addition, the “macros” feature flag enables the `#[tokio::main]` and `#[tokio::test]` attributes.

### [§](https://docs.rs/tokio/latest/tokio/#cpu-bound-tasks-and-blocking-code)CPU-bound tasks and blocking code

Tokio is able to concurrently run many tasks on a few threads by repeatedly swapping the currently running task on each thread. However, this kind of swapping can only happen at `.await` points, so code that spends a long time without reaching an `.await` will prevent other tasks from running. To combat this, Tokio provides two kinds of threads: Core threads and blocking threads.

The core threads are where all asynchronous code runs, and Tokio will by default spawn one for each CPU core. You can use the environment variable `TOKIO_WORKER_THREADS` to override the default value.

The blocking threads are spawned on demand, can be used to run blocking code that would otherwise block other tasks from running and are kept alive when not used for a certain amount of time which can be configured with [`thread_keep_alive`](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html#method.thread_keep_alive "method tokio::runtime::Builder::thread_keep_alive"). Since it is not possible for Tokio to swap out blocking tasks, like it can do with asynchronous code, the upper limit on the number of blocking threads is very large. These limits can be configured on the [`Builder`](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html "struct tokio::runtime::Builder").

To spawn a blocking task, you should use the [`spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html "fn tokio::task::spawn_blocking") function.

```
#[tokio::main]
async fn main() {
    // This is running on a core thread.

    let blocking_task = tokio::task::spawn_blocking(|| {
        // This is running on a blocking thread.
        // Blocking here is ok.
    });

    // We can wait for the blocking task like this:
    // If the blocking task panics, the unwrap below will propagate the
    // panic.
    blocking_task.await.unwrap();
}
```

If your code is CPU-bound and you wish to limit the number of threads used to run it, you should use a separate thread pool dedicated to CPU bound tasks. For example, you could consider using the [rayon](https://docs.rs/rayon) library for CPU-bound tasks. It is also possible to create an extra Tokio runtime dedicated to CPU-bound tasks, but if you do this, you should be careful that the extra runtime runs _only_ CPU-bound tasks, as IO-bound tasks on that runtime will behave poorly.

Hint: If using rayon, you can use a [`oneshot`](https://docs.rs/tokio/latest/tokio/sync/oneshot/index.html "mod tokio::sync::oneshot") channel to send the result back to Tokio when the rayon task finishes.

### [§](https://docs.rs/tokio/latest/tokio/#asynchronous-io)Asynchronous IO

As well as scheduling and running tasks, Tokio provides everything you need to perform input and output asynchronously.

The [`tokio::io`](https://docs.rs/tokio/latest/tokio/io/index.html "mod tokio::io") module provides Tokio’s asynchronous core I/O primitives, the [`AsyncRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html "trait tokio::io::AsyncRead"), [`AsyncWrite`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWrite.html "trait tokio::io::AsyncWrite"), and [`AsyncBufRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncBufRead.html "trait tokio::io::AsyncBufRead") traits. In addition, when the “io-util” feature flag is enabled, it also provides combinators and functions for working with these traits, forming as an asynchronous counterpart to [`std::io`](https://doc.rust-lang.org/nightly/std/io/index.html "mod std::io").

Tokio also includes APIs for performing various kinds of I/O and interacting with the operating system asynchronously. These include:

- [`tokio::net`](https://docs.rs/tokio/latest/tokio/net/index.html "mod tokio::net"), which contains non-blocking versions of [TCP](https://docs.rs/tokio/latest/tokio/net/tcp/index.html "mod tokio::net::tcp"), [UDP](https://docs.rs/tokio/latest/tokio/net/struct.UdpSocket.html "struct tokio::net::UdpSocket"), and [Unix Domain Sockets](https://docs.rs/tokio/latest/tokio/net/unix/index.html "mod tokio::net::unix") (enabled by the “net” feature flag),
- [`tokio::fs`](https://docs.rs/tokio/latest/tokio/fs/index.html "mod tokio::fs"), similar to [`std::fs`](https://doc.rust-lang.org/nightly/std/fs/index.html "mod std::fs") but for performing filesystem I/O asynchronously (enabled by the “fs” feature flag),
- [`tokio::signal`](https://docs.rs/tokio/latest/tokio/signal/index.html "mod tokio::signal"), for asynchronously handling Unix and Windows OS signals (enabled by the “signal” feature flag),
- [`tokio::process`](https://docs.rs/tokio/latest/tokio/process/index.html "mod tokio::process"), for spawning and managing child processes (enabled by the “process” feature flag).

## [§](https://docs.rs/tokio/latest/tokio/#examples)Examples

A simple TCP echo server:

```
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            // In a loop, read data from the socket and write the data back.
            loop {
                let n = match socket.read(&mut buf).await {
                    // socket closed
                    Ok(0) => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                // Write the data back
                if let Err(e) = socket.write_all(&buf[0..n]).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}
```

## [§](https://docs.rs/tokio/latest/tokio/#feature-flags)Feature flags

Tokio uses a set of [feature flags](https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section) to reduce the amount of compiled code. It is possible to just enable certain features over others. By default, Tokio does not enable any features but allows one to enable a subset for their use case. Below is a list of the available feature flags. You may also notice above each function, struct and trait there is listed one or more feature flags that are required for that item to be used. If you are new to Tokio it is recommended that you use the `full` feature flag which will enable all public APIs. Beware though that this will pull in many extra dependencies that you may not need.

- `full`: Enables all features listed below except `test-util` and `tracing`.
- `rt`: Enables `tokio::spawn`, the current-thread scheduler, and non-scheduler utilities.
- `rt-multi-thread`: Enables the heavier, multi-threaded, work-stealing scheduler.
- `io-util`: Enables the IO based `Ext` traits.
- `io-std`: Enable `Stdout`, `Stdin` and `Stderr` types.
- `net`: Enables `tokio::net` types such as `TcpStream`, `UnixStream` and `UdpSocket`, as well as (on Unix-like systems) `AsyncFd` and (on FreeBSD) `PollAio`.
- `time`: Enables `tokio::time` types and allows the schedulers to enable the built in timer.
- `process`: Enables `tokio::process` types.
- `macros`: Enables `#[tokio::main]` and `#[tokio::test]` macros.
- `sync`: Enables all `tokio::sync` types.
- `signal`: Enables all `tokio::signal` types.
- `fs`: Enables `tokio::fs` types.
- `test-util`: Enables testing based infrastructure for the Tokio runtime.
- `parking_lot`: As a potential optimization, use the `_parking_lot_` crate’s synchronization primitives internally. Also, this dependency is necessary to construct some of our primitives in a `const` context. `MSRV` may increase according to the `_parking_lot_` release in use.

_Note: `AsyncRead` and `AsyncWrite` traits do not require any features and are always available._

### [§](https://docs.rs/tokio/latest/tokio/#unstable-features)Unstable features

Some feature flags are only available when specifying the `tokio_unstable` flag:

- `tracing`: Enables tracing events.

Likewise, some parts of the API are only available with the same flag:

- [`task::Builder`](https://docs.rs/tokio/latest/tokio/task/struct.Builder.html "struct tokio::task::Builder")
- Some methods on [`task::JoinSet`](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html "struct tokio::task::JoinSet")
- [`runtime::RuntimeMetrics`](https://docs.rs/tokio/latest/tokio/runtime/struct.RuntimeMetrics.html "struct tokio::runtime::RuntimeMetrics")
- [`runtime::Builder::on_task_spawn`](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html#method.on_task_spawn "method tokio::runtime::Builder::on_task_spawn")
- [`runtime::Builder::on_task_terminate`](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html#method.on_task_terminate "method tokio::runtime::Builder::on_task_terminate")
- [`runtime::Builder::unhandled_panic`](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html#method.unhandled_panic "method tokio::runtime::Builder::unhandled_panic")
- [`runtime::TaskMeta`](https://docs.rs/tokio/latest/tokio/runtime/struct.TaskMeta.html "struct tokio::runtime::TaskMeta")

This flag enables **unstable** features. The public API of these features may break in 1.x releases. To enable these features, the `--cfg tokio_unstable` argument must be passed to `rustc` when compiling. This serves to explicitly opt-in to features which may break semver conventions, since Cargo [does not yet directly support such opt-ins](https://internals.rust-lang.org/t/feature-request-unstable-opt-in-non-transitive-crate-features/16193#why-not-a-crate-feature-2).

You can specify it in your project’s `.cargo/config.toml` file:

```
[build]
rustflags = ["--cfg", "tokio_unstable"]
```

The `[build]` section does **not** go in a `Cargo.toml` file. Instead it must be placed in the Cargo config file `.cargo/config.toml`.

Alternatively, you can specify it with an environment variable:

```
## Many *nix shells:
export RUSTFLAGS="--cfg tokio_unstable"
cargo build
```

```
## Windows PowerShell:
$Env:RUSTFLAGS="--cfg tokio_unstable"
cargo build
```

## [§](https://docs.rs/tokio/latest/tokio/#supported-platforms)Supported platforms

Tokio currently guarantees support for the following platforms:

- Linux
- Windows
- Android (API level 21)
- macOS
- iOS
- FreeBSD

Tokio will continue to support these platforms in the future. However, future releases may change requirements such as the minimum required libc version on Linux, the API level on Android, or the supported FreeBSD release.

Beyond the above platforms, Tokio is intended to work on all platforms supported by the mio crate. You can find a longer list [in mio’s documentation](https://crates.io/crates/mio#platforms). However, these additional platforms may become unsupported in the future.

Note that Wine is considered to be a different platform from Windows. See mio’s documentation for more information on Wine support.

### [§](https://docs.rs/tokio/latest/tokio/#wasm-support)`WASM` support

Tokio has some limited support for the `WASM` platform. Without the `tokio_unstable` flag, the following features are supported:

- `sync`
- `macros`
- `io-util`
- `rt`
- `time`

Enabling any other feature (including `full`) will cause a compilation failure.

The `time` module will only work on `WASM` platforms that have support for timers (e.g. wasm32-wasi). The timing functions will panic if used on a `WASM` platform that does not support timers.

Note also that if the runtime becomes indefinitely idle, it will panic immediately instead of blocking forever. On platforms that don’t support time, this means that the runtime can never be idle in any way.

### [§](https://docs.rs/tokio/latest/tokio/#unstable-wasm-support)Unstable `WASM` support

Tokio also has unstable support for some additional `WASM` features. This requires the use of the `tokio_unstable` flag.

Using this flag enables the use of `tokio::net` on the wasm32-wasi target. However, not all methods are available on the networking types as `WASI` currently does not support the creation of new sockets from within `WASM`. Because of this, sockets must currently be created via the `FromRawFd` trait.

` pub use task::spawn;``rt `[doc](https://docs.rs/tokio/latest/tokio/doc/index.html "mod tokio::doc")Types which are documented locally in the Tokio crate, but does not actually live here.[fs](https://docs.rs/tokio/latest/tokio/fs/index.html "mod tokio::fs")`fs`Asynchronous file utilities.[io](https://docs.rs/tokio/latest/tokio/io/index.html "mod tokio::io")Traits, helpers, and type definitions for asynchronous I/O functionality.[net](https://docs.rs/tokio/latest/tokio/net/index.html "mod tokio::net")TCP/UDP/Unix bindings for `tokio`.[process](https://docs.rs/tokio/latest/tokio/process/index.html "mod tokio::process")`process`An implementation of asynchronous process management for Tokio.[runtime](https://docs.rs/tokio/latest/tokio/runtime/index.html "mod tokio::runtime")`rt`The Tokio runtime.[signal](https://docs.rs/tokio/latest/tokio/signal/index.html "mod tokio::signal")`signal`Asynchronous signal handling for Tokio.[stream](https://docs.rs/tokio/latest/tokio/stream/index.html "mod tokio::stream")Due to the `Stream` trait’s inclusion in `std` landing later than Tokio’s 1.0 release, most of the Tokio stream utilities have been moved into the [`tokio-stream`](https://docs.rs/tokio-stream) crate.[sync](https://docs.rs/tokio/latest/tokio/sync/index.html "mod tokio::sync")`sync`Synchronization primitives for use in asynchronous contexts.[task](https://docs.rs/tokio/latest/tokio/task/index.html "mod tokio::task")Asynchronous green-threads.[time](https://docs.rs/tokio/latest/tokio/time/index.html "mod tokio::time")`time`Utilities for tracking time.[join](https://docs.rs/tokio/latest/tokio/macro.join.html "macro tokio::join")`macros`Waits on multiple concurrent branches, returning when **all** branches complete.[pin](https://docs.rs/tokio/latest/tokio/macro.pin.html "macro tokio::pin")Pins a value on the stack.[select](https://docs.rs/tokio/latest/tokio/macro.select.html "macro tokio::select")`macros`Waits on multiple concurrent branches, returning when the **first** branch completes, cancelling the remaining branches.[task\_ local](https://docs.rs/tokio/latest/tokio/macro.task_local.html "macro tokio::task_local")`rt`Declares a new task-local key of type [`tokio::task::LocalKey`](https://docs.rs/tokio/latest/tokio/task/struct.LocalKey.html "struct tokio::task::LocalKey").[try\_ join](https://docs.rs/tokio/latest/tokio/macro.try_join.html "macro tokio::try_join")`macros`Waits on multiple concurrent branches, returning when **all** branches complete with `Ok(_)` or on the first `Err(_)`.[main](https://docs.rs/tokio/latest/tokio/attr.main.html "attr tokio::main")`rt` and `macros`Marks async function to be executed by the selected runtime. This macro helps set up a `Runtime` without requiring the user to use [Runtime](https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html) or [Builder](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html) directly.[test](https://docs.rs/tokio/latest/tokio/attr.test.html "attr tokio::test")`rt` and `macros`Marks async function to be executed by runtime, suitable to test environment. This macro helps set up a `Runtime` without requiring the user to use [Runtime](https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html) or [Builder](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html) directly.
