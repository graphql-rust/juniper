Context
=======

_Context_ is a feature in [Juniper] that lets [field][4] resolvers access global data, most commonly database connections or authentication information.

Let's say that we have a simple `User`s database in a `HashMap`:
```rust
# #![allow(dead_code)]
# use std::collections::HashMap;
#
struct Database {
    users: HashMap<i32, User>,
}

struct User {
    id: i32,
    name: String,
    friend_ids: Vec<i32>,
}
#
# fn main() {}
```
We would like a `friends` [field][4] on `User` that returns a list of `User` [objects][4]. In order to write such a [field][4] though, the `Database` must be queried. To solve this, we mark the `Database` as a valid context type and assign it to the `User` [object][4]. To gain access to the context, we need to specify an argument with the same type as the specified `context` for the type:
```rust
# extern crate juniper;
# use std::collections::HashMap;
# use juniper::graphql_object;
#
struct Database {
    users: HashMap<i32, User>,
}

// Mark the `Database` as a valid context type for Juniper.
impl juniper::Context for Database {}

struct User {
    id: i32,
    name: String,
    friend_ids: Vec<i32>,
}

#[graphql_object]
#[graphql(context = Database)] // assign `Database` as the context type
impl User {
    // Inject the `Database` context by specifying an argument with the 
    // context type: 
    // - the type must be a reference;
    // - the name of the argument SHOULD be `context` (or `ctx`).
    fn friends<'db>(&self, context: &'db Database) -> Vec<&'db User> {
        //                 ^^^^^^^ or `ctx`, up to your preference
        self.friend_ids.iter()
            .map(|id| {
                context.users.get(&id).expect("could not find `User` with ID")
            })
            .collect()
    }

    fn friend<'db>(
        &self, 
        id: i32, 
        // Alternatively, the context argument may be marked with an attribute,
        // and thus, named arbitrary.
        #[graphql(context)] db: &'db Database,
        //        ^^^^^^^ or `ctx`, up to your preference
    ) -> Option<&'db User> {
        self.friend_ids.contains(&id).then(|| {
            db.users.get(&id).expect("could not find `User` with ID")
        })
    }

    fn name(&self) -> &str { 
        self.name.as_str() 
    }

    fn id(&self) -> i32 { 
        self.id 
    }
}
#
# fn main() {}
```


### Mutating and mutable references

Context cannot be specified by a mutable reference, because concurrent [fields][4] resolving may be performed. If there is something in a context that requires access by a mutable reference, the context type should follow the [_interior mutability_ pattern][5] for that (e.g. using `RwLock`, `RefCell` or similar).

For example, when using async runtime with [work stealing][6] (like [`tokio`]), which obviously requires thread safety in addition, we will need to use a corresponding async version of `RwLock`:
```rust
# extern crate juniper;
# extern crate tokio;
# use std::collections::HashMap;
# use juniper::graphql_object;
use tokio::sync::RwLock;

struct Database {
    requested_count: HashMap<i32, i32>,
}

// Since we cannot directly implement `juniper::Context`
// for `RwLock`, we use the newtype idiom.
struct DatabaseContext(RwLock<Database>);

impl juniper::Context for DatabaseContext {}

struct User {
    id: i32,
    name: String
}

#[graphql_object]
#[graphql(context = DatabaseContext)]
impl User {
    async fn times_requested<'db>(&self, ctx: &'db DatabaseContext) -> i32 {
        // Acquire a mutable reference and `.await` if async `RwLock` is used,
        // which is necessary if context consists of async operations like 
        // querying remote databases.
        
        // Obtain base type.
        let DatabaseContext(db) = ctx;
        // If context is immutable use `.read()` on `RwLock` instead.
        let mut db = db.write().await;
        
        // Perform a mutable operation.
        db.requested_count
            .entry(self.id)
            .and_modify(|e| *e += 1)
            .or_insert(1)
            .clone()
    }

    fn name(&self) -> &str { 
        self.name.as_str() 
    }

    fn id(&self) -> i32 { 
        self.id 
    }
}
#
# fn main() {}
```
> **TIP**: Replace `tokio::sync::RwLock` with `std::sync::RwLock` (or similar) if you don't intend to use async resolving.




[`tokio`]: https://docs.rs/tokio
[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org

[0]: https://spec.graphql.org/October2021#sec-Objects
[4]: https://spec.graphql.org/October2021#sec-Language.Fields
[5]: https://doc.rust-lang.org/reference/interior-mutability.html#interior-mutability
[6]: https://en.wikipedia.org/wiki/Work_stealing
