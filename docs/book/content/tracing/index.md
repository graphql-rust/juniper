# Tracing

Juniper relies on the [tracing](https://crates.io/crates/tracing) crate for instrumentation.

!FILENAME Cargo.toml

```toml
[dependencies]
tracing = "0.1.17"
tracing-subscriber = "0.2.9"
tracing-log = "0.1.1"
```

## Usage

```rust
# extern crate tracing;
# extern crate tracing_subscriber;
# extern crate tracing_log;
fn main() {
    // compatibility with the log crate (unstable)
    // converts all log records into tracing events
    tracing_log::LogTracer::init().expect("LogTracer init failed");

    // a builder for `FmtSubscriber`.
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE
        // (e.g, debug, info, warn, etc.) will be written to stdout.
        .with_max_level(tracing::Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default tracing subscriber failed");

    // ...
}
```
