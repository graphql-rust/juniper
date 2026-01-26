Implicit and explicit `null`
============================

> [GraphQL] has two semantically different ways to represent the lack of a value:
> - Explicitly providing the literal value: **null**.
> - Implicitly not providing a value at all.

There are two ways that a client can submit a [`null` value][0] as an [argument][5] or a [field][4] in a [GraphQL] query:
1. Either use an explicit `null` literal:
   ```graphql
   {
     field(arg: null)
   }
   ```
2. Or simply omit the [argument][5], so the implicit default `null` value kicks in:
   ```graphql
   {
     field
   }
   ```

There are some situations where it's useful to know which one exactly has been provided.

For example, let's say we have a function that allows users to perform a "patch" operation on themselves. Let's say our users can optionally have favorite and least favorite numbers, and the input for that might look like this:
```rust
/// Updates user attributes. Fields that are [`None`] are left as-is.
struct UserPatch {
    /// If [`Some`], updates the user's favorite number.
    favorite_number: Option<Option<i32>>,

    /// If [`Some`], updates the user's least favorite number.
    least_favorite_number: Option<Option<i32>>,
}
#
# fn main() {}
```

To set a user's favorite number to 7, we would set `favorite_number` to `Some(Some(7))`. In [GraphQL], that might look like this:
```graphql
mutation { patchUser(patch: { favoriteNumber: 7 }) }
```

To unset the user's favorite number, we would set `favorite_number` to `Some(None)`. In [GraphQL], that might look like this:
```graphql
mutation { patchUser(patch: { favoriteNumber: null }) }
```

And if we want to leave the user's favorite number alone, just set it to `None`. In [GraphQL], that might look like this:
```graphql
mutation { patchUser(patch: {}) }
```

The last two cases rely on being able to distinguish between [explicit and implicit `null`][1].

Unfortunately, plain `Option` is not capable to distinguish them. That's why in [Juniper], this can be done using the [`Nullable`] type:
```rust
# extern crate juniper;
use juniper::{FieldResult, GraphQLInputObject, Nullable, graphql_object};

#[derive(GraphQLInputObject)]
struct UserPatchInput {
    favorite_number: Nullable<i32>,
    least_favorite_number: Nullable<i32>,
}

impl From<UserPatchInput> for UserPatch {
   fn from(input: UserPatchInput) -> Self {
      Self {
         // The `explicit()` function transforms the `Nullable` into an
         // `Option<Option<T>>` as expected by the business logic layer.
         favorite_number: input.favorite_number.explicit(),
         least_favorite_number: input.least_favorite_number.explicit(),
      }
   }
}

# struct UserPatch {
#     favorite_number: Option<Option<i32>>,
#     least_favorite_number: Option<Option<i32>>,
# }
#
# struct Session;
# impl Session {
#     fn patch_user(&self, _patch: UserPatch) -> FieldResult<()> { Ok(()) }
# }
#
struct Context {
    session: Session,
}
impl juniper::Context for Context {}

struct Mutation;

#[graphql_object]
#[graphql(context = Context)]
impl Mutation {
    fn patch_user(patch: UserPatchInput, ctx: &Context) -> FieldResult<bool> {
        ctx.session.patch_user(patch.into())?;
        Ok(true)
    }
}
#
# fn main() {}
```




[`Nullable`]: https://docs.rs/juniper/0.17.1/juniper/enum.Nullable.html
[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org

[0]: https://spec.graphql.org/October2021#sec-Null-Value
[1]: https://spec.graphql.org/October2021#sel-EAFdRDHAAEJDAoBxzT
[4]: https://spec.graphql.org/October2021#sec-Language.Fields
[5]: https://spec.graphql.org/October2021#sec-Language.Arguments
