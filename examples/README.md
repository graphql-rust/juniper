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
basic_subscriptions
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

