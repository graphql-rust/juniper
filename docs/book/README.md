# Juniper Book

Book containing the Juniper documentation.

## Contributing

### Requirements

The book is built with [mdBook](https://github.com/rust-lang-nursery/mdBook).

You can install it with:

```bash
cargo install mdbook
```

### Starting a local test server

To launch a local test server that continually re-builds the book and autoreloads the page, run:

```bash
mdbook serve
```

### Building the book

You can build the book to rendered HTML with this command:

```bash
mdbook build
```

The output will be in the `./_rendered` directory.

### Running the tests

To run the tests validating all code examples in the book, run:

```bash
cd ./tests
cargo test
```

## Test setup

All Rust code examples in the book are compiled on the CI.

This is done using the [skeptic](https://github.com/budziq/rust-skeptic) library.
