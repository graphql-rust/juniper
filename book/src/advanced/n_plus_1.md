N+1 problem
===========

A common issue with [GraphQL] server implementations is how the [resolvers][2] query their datasource. With a naive and straightforward approach we quickly run into the N+1 problem, resulting in a large number of unnecessary database queries or [HTTP] requests.

```rust
# extern crate anyhow;
# extern crate juniper;
# use anyhow::anyhow;
# use juniper::{graphql_object, GraphQLObject};
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
#     async fn load_all_persons(&self) -> anyhow::Result<Vec<Person>> { unimplemented!() }
# }
#
#[derive(GraphQLObject)]
struct Cult {
    id: CultId,
    name: String,
}

struct Person {
    id: UserId,
    name: String,
    cult_id: CultId,
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
        // Effectively performs the following SQL query:
        // SELECT id, name FROM cults WHERE id = ${cult_id} LIMIT 1
        repo.load_cult_by_id(self.cult_id)
            .await?
            .ok_or_else(|| anyhow!("No cult exists for ID `{}`", self.cult_id))
    }
}

struct Query;

#[graphql_object]
#[graphql(context = Repository)]
impl Query {
    async fn persons(#[graphql(ctx)] repo: &Repository) -> anyhow::Result<Vec<Person>> {
        // Effectively performs the following SQL query:
        // SELECT id, name, cult_id FROM persons
        repo.load_all_persons().await
    }
}
```

Let's say we want to list a bunch of `cult`s `persons` were in:
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

Once the `persons` [list][1] has been [resolved][2], a separate [SQL] query is run to find the `cult` of each `Person`. We can see how this could quickly become a problem.
```sql
SELECT id, name, cult_id FROM persons;
SELECT id, name FROM cults WHERE id = 1;
SELECT id, name FROM cults WHERE id = 2;
SELECT id, name FROM cults WHERE id = 1;
SELECT id, name FROM cults WHERE id = 3;
SELECT id, name FROM cults WHERE id = 4;
SELECT id, name FROM cults WHERE id = 1;
SELECT id, name FROM cults WHERE id = 2;
-- and so on...
```

There are several ways how this problem may be resolved in [Juniper]. The most common ones are:
- [DataLoader](dataloader.md)
- Look-ahead machinery
- Eager loading




[GraphQL]: https://graphql.org
[HTTP]: https://en.wikipedia.org/wiki/HTTP
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org
[SQL]: https://en.wikipedia.org/wiki/SQL

[1]: https://spec.graphql.org/October2021#sec-List
[2]: https://spec.graphql.org/October2021#sec-Executing-Fields
