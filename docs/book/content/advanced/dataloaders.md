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

A common solution to this is to introduce a **dataloader**.
This can be done with Juniper using the crate [cksac/dataloader-rs](https://github.com/cksac/dataloader-rs), which has two types of dataloaders; cached and non-cached. This example will explore the non-cached option.


### What does it look like?

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
    ctx.cult_loader.load(id).await.unwrap()
  }
}

```

### How do I call them?

Once created, a dataloader has the functions `.load()` and `.load_many()`.
When called these return a Future.
In the above example `cult_loader.load(id: i32)` returns `Future<Cult>`. If  we had used `cult_loader.load_may(Vec<i32>)` it would have returned `Future<Vec<Cult>>`.


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
    let mut rt = futures::executor::LocalPool::new();

    // Context setup
    let cult_loader = get_loader();
    let ctx = Context::new(cult_loader);

    // Execute
    let future_execute = data.execute(&st, &ctx); 
    let res = rt.run_until(future_execute);
    let json = serde_json::to_string(&res).map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(json))
}
```

### Further Example:

For a full example using Dataloaders and Context check out [jayy-lmao/rust-graphql-docker](https://github.com/jayy-lmao/rust-graphql-docker).
