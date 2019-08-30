# 上下文

> [types/objects/using_contexts.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/types/objects/using_contexts.md)
> <br />
> commit 29025e6cae4a249fa56017dcf16b95ee4e89363e

上下文类型是 Juniper 中的一个特性，它允许字段解析器访问全局数据，最常见的是数据库连接或身份验证信息。上下文通常由 _上下文工厂（context factory）_ 方法创建。上下文的定义，与你正在使用的框架如何集成有关，请查阅 [Iron](../../servers/iron.md) 或 [Rocket](../../servers/rocket.md) 等框架的集成文档。

本章中，将向你展示如何定义上下文类型，以及如何在字段解析器中使用它。假定有一个简单的用户资料库封装在 `HashMap` 中：

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

我们希望 `User` 上的 `friends` 字段返回 `User` 对象列表。为了编写这段代码，必须查询数据库。

为了解决这个问题，我们标记 `Database` 为一个有效的上下文类型，并将其指派给 user 对象。

为了访问上下文，我们需要为被访问的上下文类型指定一个参数，此参数和被访问的 `上下文（Context）` 类型一致：

```rust
# use std::collections::HashMap;
extern crate juniper;

// 此结构体即为将要被访问的上下文
struct Database {
    users: HashMap<i32, User>,
}

// 标记 Database 为一个有效的 Juniper 上下文类型
impl juniper::Context for Database {}

struct User {
    id: i32,
    name: String,
    friend_ids: Vec<i32>,
}


// 指派 Database 作为 User 的上下文类型
#[juniper::object(
    Context = Database,
)]
impl User {
    // 3. 通过给上下文类型指定参数来注入上下文
    // 注意：
    //   - 类型必须是一个 Rust 引用
    //   - 参数名必须是 context
    fn friends(&self, context: &Database) -> Vec<&User> {

        // 5. 使用 database 查找 users
        self.friend_ids.iter()
            .map(|id| context.users.get(id).expect("无法找到匹配该 ID 的用户"))
            .collect()
    }

    fn name(&self) -> &str { 
        self.name.as_str() 
    }

    fn id(&self) -> i32 { 
        self.id 
    }
}

# fn main() { }
```

你仅获得对上下文的不可变引用，因此，如果你想要执行更改操作，你将需要利用[内部可变性（interior
mutability）](https://doc.rust-lang.org/book/first-edition/mutability.html#interior-vs-exterior-mutability)，例如：`RwLock` 或 `RefCell`。
