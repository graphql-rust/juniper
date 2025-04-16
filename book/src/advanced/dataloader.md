DataLoader
==========

DataLoader pattern, named after the correspondent [`dataloader` NPM package][0], represents a mechanism of  batching and caching data requests in a delayed manner for solving the [N+1 problem](n_plus_1.md).

> A port of the "Loader" API originally developed by [@schrockn] at Facebook in 2010 as a simplifying force to coalesce the sundry key-value store back-end APIs which existed at the time. At Facebook, "Loader" became one of the implementation details of the "Ent" framework, a privacy-aware data entity loading and caching layer within web server product code. This ultimately became the underpinning for Facebook's GraphQL server implementation and type definitions.

In [Rust] ecosystem, DataLoader pattern is introduced with the [`dataloader` crate][1], naturally usable with [Juniper].

Let's remake our [example of N+1 problem](n_plus_1.md), so it's solved by applying the DataLoader pattern:
```rust
# extern crate anyhow;
# extern crate dataloader;
# extern crate juniper;
# use std::{collections::HashMap, sync::Arc};
# use anyhow::anyhow;
# use dataloader::non_cached::Loader;
# use juniper::{GraphQLObject, graphql_object};
#
# type CultId = i32;
# type UserId = i32;
#
# struct Repository;
#
# impl Repository {
#     async fn load_cults_by_ids(&self, cult_ids: &[CultId]) -> anyhow::Result<HashMap<CultId, Cult>> { unimplemented!() }
#     async fn load_all_persons(&self) -> anyhow::Result<Vec<Person>> { unimplemented!() }
# }
#
struct Context {
    repo: Repository,
    cult_loader: CultLoader,
}

impl juniper::Context for Context {}

#[derive(Clone, GraphQLObject)]
struct Cult {
    id: CultId,
    name: String,
}

struct CultBatcher {
    repo: Repository,
}

// Since `BatchFn` doesn't provide any notion of fallible loading, like 
// `try_load()` returning `Result<HashMap<K, V>, E>`, we handle possible
// errors as loaded values and unpack them later in the resolver.
impl dataloader::BatchFn<CultId, Result<Cult, Arc<anyhow::Error>>> for CultBatcher {
    async fn load(
        &mut self, 
        cult_ids: &[CultId],
    ) -> HashMap<CultId, Result<Cult, Arc<anyhow::Error>>> {
        // Effectively performs the following SQL query:
        // SELECT id, name FROM cults WHERE id IN (${cult_id1}, ${cult_id2}, ...)
        match self.repo.load_cults_by_ids(cult_ids).await {
            Ok(found_cults) => {
                found_cults.into_iter().map(|(id, cult)| (id, Ok(cult))).collect()
            }
            // One could choose a different strategy to deal with fallible loads,
            // like consider values that failed to load as absent, or just panic.
            // See cksac/dataloader-rs#35 for details:
            // https://github.com/cksac/dataloader-rs/issues/35
            Err(e) => {
                // Since `anyhow::Error` doesn't implement `Clone`, we have to
                // work around here.
                let e = Arc::new(e);
                cult_ids.iter().map(|k| (k.clone(), Err(e.clone()))).collect()
            }
        }
    }
}

type CultLoader = Loader<CultId, Result<Cult, Arc<anyhow::Error>>, CultBatcher>;

fn new_cult_loader(repo: Repository) -> CultLoader {
    CultLoader::new(CultBatcher { repo })
        // Usually a `Loader` will coalesce all individual loads which occur 
        // within a single frame of execution before calling a `BatchFn::load()`
        // with all the collected keys. However, sometimes this behavior is not
        // desirable or optimal (perhaps, a request is expected to be spread out
        // over a few subsequent ticks).
        // A larger yield count will allow more keys to be appended to the batch,
        // but will wait longer before the actual load. For more details see:
        // https://github.com/cksac/dataloader-rs/issues/12 
        // https://github.com/graphql/dataloader#batch-scheduling
        .with_yield_count(100)
}

struct Person {
    id: UserId,
    name: String,
    cult_id: CultId,
}

#[graphql_object]
#[graphql(context = Context)]
impl Person {
    fn id(&self) -> CultId {
        self.id
    }
    
    fn name(&self) -> &str {
        self.name.as_str()
    }
    
    async fn cult(&self, ctx: &Context) -> anyhow::Result<Cult> {
        ctx.cult_loader
            // Here, we don't run the `CultBatcher::load()` eagerly, but rather
            // only register the `self.cult_id` value in the `cult_loader` and
            // wait for other concurrent resolvers to do the same.
            // The actual batch loading happens once all the resolvers register 
            // their IDs and there is nothing more to execute. 
            .try_load(self.cult_id)
            .await
            // The outer error is the `io::Error` returned by `try_load()` if
            // no value is present in the `HashMap` for the specified 
            // `self.cult_id`, meaning that there is no `Cult` with such ID
            // in the `Repository`.
            .map_err(|_| anyhow!("No cult exists for ID `{}`", self.cult_id))?
            // The inner error is the one returned by the `CultBatcher::load()`
            // if the `Repository::load_cults_by_ids()` fails, meaning that
            // running the SQL query failed.
            .map_err(|arc_err| anyhow!("{arc_err}"))
    }
}

struct Query;

#[graphql_object]
#[graphql(context = Context)]
impl Query {
    async fn persons(ctx: &Context) -> anyhow::Result<Vec<Person>> {
        // Effectively performs the following SQL query:
        // SELECT id, name, cult_id FROM persons
        ctx.repo.load_all_persons().await
    }
}

fn main() {
    
}
```

And now, performing a [GraphQL query which lead to N+1 problem](n_plus_1.md)
```graphql
query {
  persons {
    id
    name
    cult {
      id
      name
    }
  }
}
```
will lead to efficient [SQL] queries, just as expected:
```sql
SELECT id, name, cult_id FROM persons;
SELECT id, name FROM cults WHERE id IN (1, 2, 3, 4);
```




## Caching

[`dataloader::cached`] provides a [memoization][2] cache: after `BatchFn::load()` is called once with given keys, the resulting values are cached to eliminate redundant loads.

DataLoader caching does not replace [Redis], [Memcached], or any other shared application-level cache. DataLoader is first and foremost a data loading mechanism, and its cache only serves the purpose of not repeatedly loading the same data [in the context of a single request][3].

> **WARNING**: A DataLoader should be created per-request to avoid risk of bugs where one client is able to load cached/batched data from another client outside its authenticated scope. Creating a DataLoader within an individual resolver will prevent batching from occurring and will nullify any benefits of it.




## Full example

For a full example using DataLoaders in [Juniper] check out the [`jayy-lmao/rust-graphql-docker` repository][4].




[`dataloader::cached`]: https://docs.rs/dataloader/latest/dataloader/cached/index.html
[@schrockn]: https://github.com/schrockn
[Juniper]: https://docs.rs/juniper
[Memcached]: https://memcached.org
[Redis]: https://redis.io
[Rust]: https://www.rust-lang.org
[SQL]: https://en.wikipedia.org/wiki/SQL

[0]: https://github.com/graphql/dataloader
[1]: https://docs.rs/crate/dataloader
[2]: https://en.wikipedia.org/wiki/Memoization
[3]: https://github.com/graphql/dataloader#caching
[4]: https://github.com/jayy-lmao/rust-graphql-docker
