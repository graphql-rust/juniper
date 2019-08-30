# 联合

> [types/unions.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/types/unions.md)
> <br />
> commit 693405afa5a86df3a2277065696e7c42306ff630

从服务器视觉看，GraphQL 联合类似于接口：唯一的例外是联合自身不包含字段。

在Juniper中，`graphql_union!` 与[接口宏（interface
macro）](interfaces.md)具有相同的语法，但不支持定义字段。因此，接口中关于特性、占位符类型，或枚举使用，同样适用于联合。

如果查阅和[接口章节](interfaces.md)相同的示例，我们将看到相似性和折衷性：

## 特性（Traits）

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
    // 向下转型方法，每个具体类都需要实现
    fn as_human(&self) -> Option<&Human> { None }
    fn as_droid(&self) -> Option<&Droid> { None }
}

impl Character for Human {
    fn as_human(&self) -> Option<&Human> { Some(&self) }
}

impl Character for Droid {
    fn as_droid(&self) -> Option<&Droid> { Some(&self) }
}

juniper::graphql_union!(<'a> &'a Character: () as "Character" where Scalar = <S> |&self| { 
    instance_resolvers: |_| {
        // 左边表示具体类型 T，右边是返回 Option<T> 的表达式
        &Human => self.as_human(),
        &Droid => self.as_droid(),
    }
});

# fn main() {}
```

### 使用数据库查找

有毛病：此例代码还不能编译

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

juniper::graphql_union!(<'a> &'a Character: Database as "Character" where Scalar = <S> |&self| {
    instance_resolvers: |&context| {
        &Human => context.humans.get(self.id()),
        &Droid => context.droids.get(self.id()),
    }
});

# fn main() {}
```

## 占位符（placeholder）对象

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

struct Character {
    id: String,
}

juniper::graphql_union!(Character: Database where Scalar = <S> |&self| {
    instance_resolvers: |&context| {
        &Human => context.humans.get(&self.id),
        &Droid => context.droids.get(&self.id),
    }
});

# fn main() {}
```

## 枚举

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

juniper::graphql_union!(Character: () where Scalar = <S> |&self| {
    instance_resolvers: |_| {
        &Human => match *self { Character::Human(ref h) => Some(h), _ => None },
        &Droid => match *self { Character::Droid(ref d) => Some(d), _ => None },
    }
});

# fn main() {}
```
