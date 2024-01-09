# Avoiding the N+1 Problem With Dataloaders

A common issue with graphql servers is how the resolvers query their datasource.
This issue results in a large number of unnecessary database queries or http requests.
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

A common solution to this is to introduce a **dataloader**.
This can be done with Juniper using the crate [cksac/dataloader-rs](https://github.com/cksac/dataloader-rs), which has two types of dataloaders; cached and non-cached.

#### Cached Loader
DataLoader provides a memoization cache, after .load() is called once with a given key, the resulting value is cached to eliminate redundant loads.

DataLoader caching does not replace Redis, Memcache, or any other shared application-level cache. DataLoader is first and foremost a data loading mechanism, and its cache only serves the purpose of not repeatedly loading the same data in the context of a single request to your Application. [(read more)](https://github.com/graphql/dataloader#caching)

### What does it look like?

!FILENAME Cargo.toml

```toml
[dependencies]
actix-identity = "0.4.0-beta.4"
actix-rt = "1.0"
actix-web = "2.0"
async-trait = "0.1.30"
dataloader = "0.12.0"
futures = "0.3"
juniper = "0.16.0"
postgres = "0.15.2"
```

```rust, ignore
// use dataloader::cached::Loader;
use dataloader::non_cached::Loader;
use dataloader::BatchFn;
use std::collections::HashMap;
use postgres::{Connection, TlsMode};
use std::env;

pub fn get_db_conn() -> Connection {
    let pg_connection_string = env::var("DATABASE_URI").expect("need a db uri");
    println!("Connecting to {pg_connection_string}");
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

#[async_trait]
impl BatchFn<i32, Cult> for CultBatcher {

    // A hashmap is used, as we need to return an array which maps each original key to a Cult.
    async fn load(&self, keys: &[i32]) -> HashMap<i32, Cult> {
        println!("load cult batch {keys:?}");
        let mut cult_hashmap = HashMap::new();
        get_cult_by_ids(&mut cult_hashmap, keys.to_vec());
        cult_hashmap
    }
}

pub type CultLoader = Loader<i32, Cult, CultBatcher>;

// To create a new loader
pub fn get_loader() -> CultLoader {
    Loader::new(CultBatcher)
      // Usually a DataLoader will coalesce all individual loads which occur 
      // within a single frame of execution before calling your batch function with all requested keys.
      // However sometimes this behavior is not desirable or optimal. 
      // Perhaps you expect requests to be spread out over a few subsequent ticks
      // See: https://github.com/cksac/dataloader-rs/issues/12 
      // More info: https://github.com/graphql/dataloader#batch-scheduling 
      // A larger yield count will allow more requests to append to batch but will wait longer before actual load.
      .with_yield_count(100)
}

#[juniper::graphql_object(Context = Context)]
impl Cult {
  //  your resolvers

  // To call the dataloader 
  pub async fn cult_by_id(ctx: &Context, id: i32) -> Cult {
    ctx.cult_loader.load(id).await
  }
}

```

### How do I call them?

Once created, a dataloader has the async functions `.load()` and `.load_many()`.
In the above example `cult_loader.load(id: i32).await` returns `Cult`. If  we had used `cult_loader.load_many(Vec<i32>).await` it would have returned `Vec<Cult>`.


### Where do I create my dataloaders?

**Dataloaders** should be created per-request to avoid risk of bugs where one user is able to load cached/batched data from another user/ outside of its authenticated scope.
Creating dataloaders within individual resolvers will prevent batching from occurring and will nullify the benefits of the dataloader.

For example:

_When you declare your context_
```rust, ignore
use juniper;

#[derive(Clone)]
pub struct Context {
    pub cult_loader: CultLoader,
}

impl juniper::Context for Context {}

impl Context {
    pub fn new(cult_loader: CultLoader) -> Self {
        Self {
            cult_loader
        }
    }
}
```

_Your handler for GraphQL (Note: instantiating context here keeps it per-request)_
```rust, ignore
pub async fn graphql(
    st: web::Data<Arc<Schema>>,
    data: web::Json<GraphQLRequest>,
) -> Result<HttpResponse, Error> {

    // Context setup
    let cult_loader = get_loader();
    let ctx = Context::new(cult_loader);

    // Execute
    let res = data.execute(&st, &ctx).await; 
    let json = serde_json::to_string(&res).map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(json))
}
```

### Further Example:

For a full example using Dataloaders and Context check out [jayy-lmao/rust-graphql-docker](https://github.com/jayy-lmao/rust-graphql-docker).
