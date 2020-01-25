# Objects and generics

Yet another point where GraphQL and Rust differs is in how generics work. In
Rust, almost any type could be generic - that is, take type parameters. In
GraphQL, there are only two generic types: lists and non-nullables.

This poses a restriction on what you can expose in GraphQL from Rust: no generic
structs can be exposed - all type parameters must be bound. For example, you can
not make e.g. `Result<T, E>` into a GraphQL type, but you _can_ make e.g.
`Result<User, String>` into a GraphQL type.

Let's make a slightly more compact but generic implementation of [the last
chapter](non_struct_objects.md):

```rust
# #[derive(juniper::GraphQLObject)] struct User { name: String }
# #[derive(juniper::GraphQLObject)] struct ForumPost { title: String }

#[derive(juniper::GraphQLObject)]
struct ValidationError {
    field: String,
    message: String,
}

# #[allow(dead_code)]
struct MutationResult<T>(Result<T, Vec<ValidationError>>);

#[juniper::graphql_object(
    name = "UserResult",
)]
impl MutationResult<User> {
    fn user(&self) -> Option<&User> {
        self.0.as_ref().ok()
    }

    fn error(&self) -> Option<&Vec<ValidationError>> {
        self.0.as_ref().err()
    }
}

#[juniper::graphql_object(
    name = "ForumPostResult",
)]
impl MutationResult<ForumPost> {
    fn forum_post(&self) -> Option<&ForumPost> {
        self.0.as_ref().ok()
    }

    fn error(&self) -> Option<&Vec<ValidationError>> {
        self.0.as_ref().err()
    }
}

# fn main() {}
```

Here, we've made a wrapper around `Result` and exposed some concrete
instantiations of `Result<T, E>` as distinct GraphQL objects. The reason we
needed the wrapper is of Rust's rules for when you can derive a trait - in this
case, both `Result` and Juniper's internal GraphQL trait are from third-party
sources.

Because we're using generics, we also need to specify a name for our
instantiated types. Even if Juniper _could_ figure out the name,
`MutationResult<User>` wouldn't be a valid GraphQL type name.
