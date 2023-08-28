# Juniper Examples

This directory contains examples of how to use Juniper.

## How to run

To run an example, you need to have a working Rust toolchain installed. You can
get it from [rustup](https://rustup.rs/).

Then, you can run the example using its workspace:

```bash
cargo run --example <example_name>
```

Where `<example_name>` is one of the following workspace members:

```
actix_server
hyper_server
iron_server
rocket_server
warp_server
```

e.g. to run the `actix_server` example:

```bash
cargo run --example actix_server
```

You can also run an example directly from an `examples` workspace directory. To
run the `actix_server` example:

```bash
cd examples/actix_subscriptions
cargo run
     Finished dev [unoptimized + debuginfo] target(s) in 0.13s
     Running `/path/to/repo/juniper/target/debug/example_actix_subscriptions`
[2022-11-20T07:46:08Z INFO  actix_server::builder] Starting 10 workers
[2022-11-20T07:46:08Z INFO  actix_server::server] Actix runtime found; starting in Actix runtime
```

Note if you want to run the code within your own project, you need to change
the relative paths in `Cargo.toml`, e.g:

```toml
juniper_graphql_ws = { path = "../../juniper_graphql_ws" }
```

to:

```toml
juniper_graphql_ws = "0.3.0"
```

