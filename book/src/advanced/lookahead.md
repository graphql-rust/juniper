Look-ahead
==========

> In backtracking algorithms, **look ahead** is the generic term for a subprocedure that attempts to foresee the effects of choosing a branching variable to evaluate one of its values. The two main aims of look-ahead are to choose a variable to evaluate next and to choose the order of values to assign to it.

In [GraphQL], look-ahead machinery allows us to introspect the currently [executed][1] [GraphQL operation][2] to see which [fields][3] has been actually selected by it.

In [Juniper], it's represented by the [`Executor::look_ahead()`][20] method.
```rust
# extern crate juniper;
# use juniper::{graphql_object, Executor, GraphQLObject, ScalarValue};
#
# type UserId = i32;
#
#[derive(GraphQLObject)]
struct Person {
    id: UserId,
    name: String,
}

struct Query;

#[graphql_object]
// NOTICE: Specifying `ScalarValue` as custom named type parameter,
//         so its name is similar to the one used in methods.
#[graphql(scalar = S: ScalarValue)]
impl Query {
    fn persons<S: ScalarValue>(executor: &Executor<'_, '_, (), S>,) -> Vec<Person> {
        // Let's see which `Person`'s fields were selected in the client query. 
        for field_name in executor.look_ahead().children().names() {
            dbg!(field_name);
        }
        // ...
#       unimplemented!()
    }
}
```
> **TIP**: `S: ScalarValue` type parameter on the method is required here to keep the [`Executor`] being generic over [`ScalarValue`] types. We, instead, could have used the [`DefaultScalarValue`], which is the default [`ScalarValue`] type for the [`Executor`], and make our code more ergonomic, but less flexible and generic.
> ```rust
> # extern crate juniper;
> # use juniper::{graphql_object, DefaultScalarValue, Executor, GraphQLObject};
> #
> # type UserId = i32;
> #
> # #[derive(GraphQLObject)]
> # struct Person {
> #     id: UserId,
> #     name: String,
> # }
> #
> # struct Query;
> #
> #[graphql_object]
> #[graphql(scalar = DefaultScalarValue)]
> impl Query {
>     fn persons(executor: &Executor<'_, '_, ()>,) -> Vec<Person> {
>         for field_name in executor.look_ahead().children().names() {
>             dbg!(field_name);
>         }
>         // ...
> #       unimplemented!()
>     }
> }
> ```




[`DefaultScalarValue`]: https://docs.rs/juniper/latest/juniper/enum.DefaultScalarValue.html
[`Executor`]: https://docs.rs/juniper/latest/juniper/executor/struct.Executor.html
[`ScalarValue`]: https://docs.rs/juniper/latest/juniper/trait.ScalarValue.html
[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org

[1]: https://spec.graphql.org/October2021#sec-Execution
[2]: https://spec.graphql.org/October2021#sec-Language.Operations\
[3]: https://spec.graphql.org/October2021#sec-Language.Fields
[20]: https://docs.rs/juniper/latest/juniper/executor/struct.Executor.html#method.look_ahead
