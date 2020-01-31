# Avoiding the N+1 Problem With Dataloaders

A common issue with graphql servers is how the resolvers query their datasource.
his issue results in a large number of unneccessary database queries or http requests.
Say you were wanting to list a bunch of cults people were in

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

What would be executed by a SQL database would be:

```sql
SELECT id, name, cult_id FROM persons;
SELECT id, name FROM cults WHERE id = 1;
SELECT id, name FROM cults WHERE id = 1;
SELECT id, name FROM cults WHERE id = 1;
SELECT id, name FROM cults WHERE id = 1;
SELECT id, name FROM cults WHERE id = 2;
SELECT id, name FROM cults WHERE id = 2;
SELECT id, name FROM cults WHERE id = 2;
# ...
```

Once the list of users has been returned, a separate query is run to find the cult of each user.
You can see how this could quickly become a problem.

A common solution to this is to introduce a **dataloader**. A dataloader batches resolver execution (and _can_ even cache it).
This can be done with Juniper using the crate [cksac/dataloader-rs](https://github.com/cksac/dataloader-rs). 

!FILENAME Cargo.toml

```toml
[dependencies]
actix-identity = "0.2"
actix-rt = "1.0"
actix-web = {version = "2.0", features = []}
juniper = { git = "https://github.com/graphql-rust/juniper", branch = "async-await", features = ["async"] }
futures = "0.3"
postgres = "0.15.2"
dataloader = "0.6.0"
```

```rust, ignore
use dataloader::Loader;
use dataloader::{BatchFn, BatchFuture};
use futures::{future, FutureExt as _};
use std::collections::HashMap;
use postgres::{Connection, TlsMode};
use std::env;

pub fn get_db_conn() -> Connection {
    let pg_connection_string = env::var("DATABASE_URI").expect("need a db uri");
    println!("Connecting to {}", pg_connection_string);
    let conn = Connection::connect(&pg_connection_string[..], TlsMode::None).unwrap();
    println!("Connection is fine");
    conn
}

#[derive(Debug, Clone)]
pub struct Cult {
  pub id: i32,
  pub name: String,
}

pub fn get_cult_by_ids(hashmap: &mut HashMap<i32, Cult>, ids: Vec<i32>) {
  let conn = get_db_conn();
  for row in &conn
    .query("SELECT id, name FROM cults WHERE id = ANY($1)", &[&ids])
    .unwrap()
  {
    let cult = Cult {
      id: row.get(0),
      name: row.get(1),
    };
    hashmap.insert(cult.id, cult);
  }
}

pub struct CultBatcher;

impl BatchFn<i32, Cult> for CultBatcher {
  type Error = ();

  fn load(&self, keys: &[i32]) -> BatchFuture<Cult, Self::Error> {
    println!("load batch {:?}", keys);
    // A hashmap is used, as we need to return an array which maps each original key to a Cult.
    let mut cult_hashmap = HashMap::new();
    get_cult_by_ids(&mut cult_hashmap, keys.to_vec());

    future::ready(keys.iter().map(|key| cult_hashmap[key].clone()).collect())
        .unit_error()
        .boxed()
  }
}

pub type CultLoader = Loader<i32, Cult, (), CultBatcher>;

// To create a new loader
pub fn get_loader() -> CultLoader {
    Loader::new(CultBatcher)
}

#[juniper::graphql_object(Context = Context)]
impl Cult {
  //  your resolvers

  // To call the dataloader 
  pub async fn cult_by_id(ctx: &Context, id: i32) -> Cult {
    ctx.cult_loader.cult_by_id.load(id).await.unwrap()
  }
}

```

As seen immediately above, when a new `PersonBatcher` is instantiated,
it can be called with `.load(id)` to return a `Future` which resolves to a value with `await`.

A **Dataloader** should be created in context, either at the server start, or on a per-request basis.
If it is created within a resolver then it will be created for each time it needs to resolve a field,
and therefore will never batch more than the single object it is evaluating the field of.

For a full example using Dataloaders and Context check out [jayy-lmao/rust-graphql-docker](https://github.com/jayy-lmao/rust-graphql-docker).
