# 接口

> [types/interfaces.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/types/interfaces.md)
> <br />
> commit 693405afa5a86df3a2277065696e7c42306ff630

GraphQL 接口能够正确地映射到常见的面向对象语言接口，如 Java 或 C#。但很不幸，Rust 并无与 GraphQL 接口正确映射的概念。因此，Juniper 中定义接口需要一点点范例代码；另一方面，可以做到让你完全控制所支持接口的类型。

为了突出展示在 Rust 中实现接口的不同方式，让我们看看不同实现方式所实现的相同结果：

## 特性（Traits）

特性（Traits）或许是你在 Rust 语言构建 GraphQL 接口时想使用的最明显概念。但是因为 GraphQL 支持`向下转型（downcasting）`而 Rust 却不支持，所以你必须手动实现如何将特性（trait）转换为具体类型。可以通过如下方式：

### 通过存取器方法向下转型

```rust
#[derive(juniper::GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

trait Character {
    fn id(&self) -> &str;

    // 向下转型方法，每个具体类都需要实现
    fn as_human(&self) -> Option<&Human> { None }
    fn as_droid(&self) -> Option<&Droid> { None }
}

impl Character for Human {
    fn id(&self) -> &str { self.id.as_str() }
    fn as_human(&self) -> Option<&Human> { Some(&self) }
}

impl Character for Droid {
    fn id(&self) -> &str { self.id.as_str() }
    fn as_droid(&self) -> Option<&Droid> { Some(&self) }
}

juniper::graphql_interface!(<'a> &'a Character: () as "Character" where Scalar = <S> |&self| {
    field id() -> &str { self.id() }

    instance_resolvers: |_| {
        // 左边表示具体类型 T，右边是返回 Option<T> 的表达式
        &Human => self.as_human(),
        &Droid => self.as_droid(),
    }
});

# fn main() {}
```

`instance_resolvers 闭包`列出了给定接口的所有实现，以及如何解析接口。

如所看到的，使用特性（traits）意义不大：你需要列出 trait 自身的所有具体类型，且会有一些重复，啰里啰唆。

### 使用数据库查找

当具体类被请求时，如果你可以提供额外的数据库查询，则可以废弃向下转型方法，转而使用上下文。如下示例代码，我们将使用两个哈希表，你可以用两张表和一些 SQL 调用来代替：

```rust
# use std::collections::HashMap;
#[derive(juniper::GraphQLObject)]
#[graphql(Context = Database)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
#[graphql(Context = Database)]
struct Droid {
    id: String,
    primary_function: String,
}

struct Database {
    humans: HashMap<String, Human>,
    droids: HashMap<String, Droid>,
}

impl juniper::Context for Database {}

trait Character {
    fn id(&self) -> &str;
}

impl Character for Human {
    fn id(&self) -> &str { self.id.as_str() }
}

impl Character for Droid {
    fn id(&self) -> &str { self.id.as_str() }
}

juniper::graphql_interface!(<'a> &'a Character: Database as "Character" where Scalar = <S> |&self| {
    field id() -> &str { self.id() }

    instance_resolvers: |&context| {
        &Human => context.humans.get(self.id()),
        &Droid => context.droids.get(self.id()),
    }
});

# fn main() {}
```

虽移除了向下转型方法，但代码仍有点啰嗦。

## 占位符（placeholder）对象

继续上段示例代码，trait 自身似乎没有必要，也许它可以仅是一个包含 ID 的结构体（struct）？

Continuing on from the last example, the trait itself seems a bit unneccesary.
Maybe it can just be a struct containing the ID?

```rust
# use std::collections::HashMap;
#[derive(juniper::GraphQLObject)]
#[graphql(Context = "Database")]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
#[graphql(Context = "Database")]
struct Droid {
    id: String,
    primary_function: String,
}

struct Database {
    humans: HashMap<String, Human>,
    droids: HashMap<String, Droid>,
}

impl juniper::Context for Database {}

struct Character {
    id: String,
}

juniper::graphql_interface!(Character: Database where Scalar = <S> |&self| {
    field id() -> &str { self.id.as_str() }

    instance_resolvers: |&context| {
        &Human => context.humans.get(&self.id),
        &Droid => context.droids.get(&self.id),
    }
});

# fn main() {}
```

减少了不少重复，但如果接口数据较多的情况下，此种做法不符合实际。

## 枚举

使用枚举和模式匹配介于使用特性（trait）和使用占位符（placeholder）对象之间。本例中，我们无需额外的数据库调用，因此移除。

```rust
#[derive(juniper::GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

# #[allow(dead_code)]
enum Character {
    Human(Human),
    Droid(Droid),
}

juniper::graphql_interface!(Character: () where Scalar = <S> |&self| {
    field id() -> &str {
        match *self {
            Character::Human(Human { ref id, .. }) |
            Character::Droid(Droid { ref id, .. }) => id,
        }
    }

    instance_resolvers: |_| {
        &Human => match *self { Character::Human(ref h) => Some(h), _ => None },
        &Droid => match *self { Character::Droid(ref d) => Some(d), _ => None },
    }
});

# fn main() {}
```
