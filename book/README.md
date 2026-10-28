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

Testing the Book also requires [jq](https://jqlang.org).


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
make test.book
```

This command builds the dependencies of the code examples with Cargo and passes the library directories to `mdbook test`.
