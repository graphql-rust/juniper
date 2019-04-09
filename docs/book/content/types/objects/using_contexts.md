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

struct Database {
    users: HashMap<i32, User>,
}

struct User {
    id: i32,
    name: String,
    friend_ids: Vec<i32>,
}

# fn main() { }
```

We would like a `friends` field on `User` that returns a list of `User` objects.
In order to write such a field though, the database must be queried.

To solve this, we mark the `Database` as a valid context type and assign it to
the user object. Then, we use the special `&executor` argument to access the
current context object:

```rust
# use std::collections::HashMap;
extern crate juniper;

struct Database {
    users: HashMap<i32, User>,
}

struct User {
    id: i32,
    name: String,
    friend_ids: Vec<i32>,
}

// 1. Mark the Database as a valid context type for Juniper
impl juniper::Context for Database {}

// 2. Assign Database as the context type for User
juniper::graphql_object!(User: Database |&self| {
    // 3. Use the special executor argument
    field friends(&executor) -> Vec<&User> {
        // 4. Use the executor to access the context object
        let database = executor.context();

        // 5. Use the database to lookup users
        self.friend_ids.iter()
            .map(|id| database.users.get(id).expect("Could not find user with ID"))
            .collect()
    }

    field name() -> &str { self.name.as_str() }
    field id() -> i32 { self.id }
});

# fn main() { }
```

You only get an immutable reference to the context, so if you want to affect
change to the execution, you'll need to use [interior
mutability](https://doc.rust-lang.org/book/first-edition/mutability.html#interior-vs-exterior-mutability)
using e.g. `RwLock` or `RefCell`.
