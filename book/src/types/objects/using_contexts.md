# Using contexts

The context type is a feature in Juniper that lets field resolvers access global
data, most commonly database connections or authentication information. The
context is usually created from a _context factory_. How this is defined is
specific to the framework integration you're using, so check out the
documentation for either the [Iron](../../servers/iron.md) or [Rocket](../../servers/rocket.md)
integration.

In this chapter, we'll show you how to define a context type and use it in field
resolvers. Let's say that we have a simple user database in a `HashMap`:

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
# fn main() { }
```

We would like a `friends` field on `User` that returns a list of `User` objects.
In order to write such a field though, the database must be queried.

To solve this, we mark the `Database` as a valid context type and assign it to
the user object. 

To gain access to the context, we need to specify an argument with the same 
type as the specified `Context` for the type:


```rust
# extern crate juniper;
# use std::collections::HashMap;
# use juniper::graphql_object;
#
// This struct represents our context.
struct Database {
    users: HashMap<i32, User>,
}

// Mark the Database as a valid context type for Juniper
impl juniper::Context for Database {}

struct User {
    id: i32,
    name: String,
    friend_ids: Vec<i32>,
}

// Assign Database as the context type for User
#[graphql_object(context = Database)]
impl User {
    // Inject the context by specifying an argument with the context type.
    // Note: 
    //   - the type must be a reference
    //   - the name of the argument SHOULD be `context`
    fn friends<'db>(&self, context: &'db Database) -> Vec<&'db User> {
        // Use the database to lookup users
        self.friend_ids.iter()
            .map(|id| context.users.get(id).expect("Could not find user with ID"))
            .collect()
    }

    fn name(&self) -> &str { 
        self.name.as_str() 
    }

    fn id(&self) -> i32 { 
        self.id 
    }
}
#
# fn main() { }
```

You only get an immutable reference to the context, so if you want to affect
change to the execution, you'll need to use [interior
mutability](https://doc.rust-lang.org/book/first-edition/mutability.html#interior-vs-exterior-mutability)
using e.g. `RwLock` or `RefCell`.




## Dealing with mutable references

Context cannot be specified by a mutable reference, because concurrent fields resolving may be performed. If you have something in your context that requires access by mutable reference, then you need to leverage the [interior mutability][1] for that.

For example, when using async runtime with [work stealing][2] (like `tokio`), which obviously requires thread safety in addition, you will need to use a corresponding async version of `RwLock`:
```rust
# extern crate juniper;
# extern crate tokio;
# use std::collections::HashMap;
# use juniper::graphql_object;
use tokio::sync::RwLock;

struct Database {
    requested_count: HashMap<i32, i32>,
}

// Since we cannot directly implement juniper::Context
// for RwLock we use the newtype idiom
struct DatabaseContext(RwLock<Database>);

impl juniper::Context for DatabaseContext {}

struct User {
    id: i32,
    name: String
}

#[graphql_object(context=DatabaseContext)]
impl User {
    async fn times_requested<'db>(&self, context: &'db DatabaseContext) -> i32 {
        // Acquire a mutable reference and await if async RwLock is used,
        // which is necessary if context consists async operations like 
        // querying remote databases.
        // Obtain base type
        let DatabaseContext(context) = context;
        // If context is immutable use .read() on RwLock.
        let mut context = context.write().await;
        // Perform a mutable operation.
        context.requested_count.entry(self.id).and_modify(|e| { *e += 1 }).or_insert(1).clone()
    }

    fn name(&self) -> &str { 
        self.name.as_str() 
    }

    fn id(&self) -> i32 { 
        self.id 
    }
}
#
# fn main() { }
```
Replace `tokio::sync::RwLock` with `std::sync::RwLock` (or similar) if you don't intend to use async resolving.




[1]: https://doc.rust-lang.org/book/ch15-05-interior-mutability.html
[2]: https://en.wikipedia.org/wiki/Work_stealing
