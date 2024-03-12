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
    fn persons<S: ScalarValue>(executor: &Executor<'_, '_, (), S>) -> Vec<Person> {
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
>     fn persons(executor: &Executor<'_, '_, ()>) -> Vec<Person> {
>         for field_name in executor.look_ahead().children().names() {
>             dbg!(field_name);
>         }
>         // ...
> #       unimplemented!()
>     }
> }
> ```




## N+1 problem

```rust
# extern crate anyhow;
# extern crate juniper;
# use std::collections::HashMap;
# use anyhow::anyhow;
# use juniper::{graphql_object, Executor, GraphQLObject, ScalarValue};
#
# type CultId = i32;
# type UserId = i32;
#
# struct Repository;
#
# impl juniper::Context for Repository {}
#
# impl Repository {
#     async fn load_cult_by_id(&self, cult_id: CultId) -> anyhow::Result<Option<Cult>> { unimplemented!() }
#     async fn load_cults_by_ids(&self, cult_ids: &[CultId]) -> anyhow::Result<HashMap<CultId, Cult>> { unimplemented!() }
#     async fn load_all_persons(&self) -> anyhow::Result<Vec<Person>> { unimplemented!() }
# }
# 
# enum Either<L, R> {
#     Absent(L),
#     Loaded(R),  
# }
#
#[derive(Clone, GraphQLObject)]
struct Cult {
    id: CultId,
    name: String,
}

struct Person {
    id: UserId,
    name: String,
    cult: Either<CultId, Cult>,
}

#[graphql_object]
#[graphql(context = Repository)]
impl Person {
    fn id(&self) -> CultId {
        self.id
    }
    
    fn name(&self) -> &str {
        self.name.as_str()
    }
    
    async fn cult(&self, #[graphql(ctx)] repo: &Repository) -> anyhow::Result<Cult> {
        match &self.cult {
            Either::Loaded(cult) => Ok(cult.clone()),
            Either::Absent(cult_id) => {
                // Effectively performs the following SQL query:
                // SELECT id, name FROM cults WHERE id = ${cult_id} LIMIT 1
                repo.load_cult_by_id(*cult_id)
                    .await?
                    .ok_or_else(|| anyhow!("No cult exists for ID `{cult_id}`"))
            }
        }
    }
}

struct Query;

#[graphql_object]
#[graphql(context = Repository, scalar = S: ScalarValue)]
impl Query {
    async fn persons<S: ScalarValue>(
        #[graphql(ctx)] repo: &Repository,
        executor: &Executor<'_, '_, Repository, S>,
    ) -> anyhow::Result<Vec<Person>> {
        // Effectively performs the following SQL query:
        // SELECT id, name, cult_id FROM persons
        let mut persons = repo.load_all_persons().await?;
        
        // If the `Person.cult` field has been requested.
        if executor.look_ahead()
            .children()
            .iter()
            .any(|sel| sel.field_original_name() == "cult") 
        {
            // Gather `Cult.id`s to load eagerly.
            let cult_ids = persons
                .iter()
                .filter_map(|p| {
                    match &p.cult {
                        Either::Absent(cult_id) => Some(*cult_id),
                        // If for some reason a `Cult` is already loaded,
                        // then just skip it.
                        Either::Loaded(_) => None,
                    }
                })
                .collect::<Vec<_>>();
            
            // Load the necessary `Cult`s eagerly.
            let cults = repo.load_cults_by_ids(&cult_ids).await?;
            
            for p in &mut persons {
                let Either::Absent(cult_id) = &p.cult else { continue; };
                p.cult = Either::Loaded(
                    cults.get(cult_id)
                        .ok_or_else(|| anyhow!("No cult exists for ID `{cult_id}`"))?
                        .clone(),
                );
            }
        }
        
        Ok(persons)
    }
}
```




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
