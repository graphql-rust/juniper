Juniper Book
============

Book containing the [`juniper`](https://docs.rs/juniper) user guide.




## Contributing


### Requirements

The Book is built with [mdBook](https://github.com/rust-lang/mdBook).

You may install it with:
```bash
cargo install mdbook
```


### Local test server

To launch a local test server that continually re-builds the Book and auto-reloads the page, run:
```bash
mdbook serve

# or from project root dir:
make book.serve
```


### Building

You may build the Book to rendered HTML with this command:
```bash
mdbook build

# or from project root dir:
make book
```

The output will be in the `_rendered/` directory.


### Testing

To run the tests validating all code examples in the book, run (from project root dir):
```bash
cargo test -p juniper_book

# or via shortcut:
make test.book
```

The `juniper_book` crate includes every chapter as the documentation of a module. `cargo test` then runs the code examples as doc tests. A new chapter needs an entry in `src/lib.rs`. A unit test fails if `src/lib.rs` and `SUMMARY.md` list different chapters.
