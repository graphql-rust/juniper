Generics
========

Yet another point where [GraphQL] and [Rust] differs is in how generics work:
- In [Rust], almost any type could be generic - that is, take type parameters. 
- In [GraphQL], there are only two generic types: [lists][1] and [non-`null`ables][2].

This poses a restriction on what we can expose in [GraphQL] from [Rust]: no generic structs can be exposed - all type parameters must be bound. For example, we cannot expose `Result<T, E>` as a [GraphQL type][0], but we _can_ expose `Result<User, String>` as a [GraphQL type][0].

Let's make a slightly more compact but generic implementation of [the last schema error example](error/schema.md#example-non-struct-objects):
```rust
# extern crate juniper;
# use juniper::{GraphQLObject, graphql_object};
#
#[derive(GraphQLObject)] 
struct User { 
    name: String, 
}

#[derive(GraphQLObject)] 
struct ForumPost { 
    title: String,
}

#[derive(GraphQLObject)]
struct ValidationError {
    field: String,
    message: String,
}

struct MutationResult<T>(Result<T, Vec<ValidationError>>);

#[graphql_object]
#[graphql(name = "UserResult")]
impl MutationResult<User> {
    fn user(&self) -> Option<&User> {
        self.0.as_ref().ok()
    }

    fn error(&self) -> Option<&[ValidationError]> {
        self.0.as_ref().err().map(Vec::as_slice)
    }
}

#[graphql_object]
#[graphql(name = "ForumPostResult")]
impl MutationResult<ForumPost> {
    fn forum_post(&self) -> Option<&ForumPost> {
        self.0.as_ref().ok()
    }

    fn error(&self) -> Option<&[ValidationError]> {
        self.0.as_ref().err().map(Vec::as_slice)
    }
}
#
# fn main() {}
```

Here, we've made a wrapper around a `Result` and exposed some concrete instantiations of `Result<T, E>` as distinct [GraphQL objects][3]. 

> **NOTE**: The reason we needed the wrapper is of [Rust]'s [orphan rules][10] (both the `Result` and [Juniper]'s internal traits are from third-party sources).

> **NOTE**: Because we're using generics, we also need to specify a `name` for our instantiated [GraphQL types][0]. Even if [Juniper] _could_ figure out the name, `MutationResult<User>` wouldn't be a [valid GraphQL type name][4]. And, also, two different [GraphQL types][0] cannot have the same `MutationResult` name, inferred by default.




[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org

[0]: https://spec.graphql.org/October2021#sec-Types
[1]: https://spec.graphql.org/October2021#sec-List
[2]: https://spec.graphql.org/October2021#sec-Non-Null
[3]: https://spec.graphql.org/October2021#sec-Objects
[4]: https://spec.graphql.org/October2021#sec-Names
[10]: https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence
