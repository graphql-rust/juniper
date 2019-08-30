# 定义对象

> [types/objects/defining_objects.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/types/objects/defining_objects.md)
> <br />
> commit 693405afa5a86df3a2277065696e7c42306ff630


尽管 Rust 中的任何类型都可以暴露为 GraphQL 对象，但最常见的类型是结构体（struct）。

Juniper 中，有两种方式创建 GraphQL对象：如果想要暴露一个简单结构体（struct），最简单的方式是自定义派生属性；另外一种方式将在[复杂字段](complex_fields.md)章节中介绍。

```rust
#[derive(juniper::GraphQLObject)]
struct Person {
    name: String,
    age: i32,
}

# fn main() {}
```

上述代码将创建一个名为 `Person` 的 GraphQL 对象，有两个字段：`String!` 类型的 `name`，`Int!` 类型的  `age`。Rust 语言类型系统中，变量绑定默认为非空（non-null）。若你需要可空（nullable）字段，可以使用 `Option<T>`。

我们应当利用 GraphQL 是自文档化（self-documenting）的特点，向类型和字段添加描述。Juniper 将自动使用关联的 Rust 文档注释作为 GraphQL 描述：

!文件名 通过 Rust 文档注释作为 GraphQL 描述

```rust
#[derive(juniper::GraphQLObject)]
/// 个人信息
struct Person {
    /// 个人全名，包括姓氏和名字
    name: String,
    /// 个人年龄，以年为单位，按月份四舍五入
    age: i32,
}

# fn main() {}
```

Rust 中不能使用文档注释的对象和字段，可通过 `graphql` 属性设置`描述`。如下示例和上述代码等价：

!文件名 通过 graphql 属性设置描述

```rust
#[derive(juniper::GraphQLObject)]
#[graphql(description="个人信息")]
struct Person {
    #[graphql(description="个人全名，包括姓氏和名字")]
    name: String,
    #[graphql(description="个人年龄，以年为单位，按月份四舍五入")]
    age: i32,
}

# fn main() {}
```

通过 `graphql` 属性设置的描述优先于 Rust 文档注释，这使得内部 Rust 文档和外部 GraphQL 文档能够不同：

```rust
#[derive(juniper::GraphQLObject)]
#[graphql(description="这段描述展示在 GraphQL")]
/// 这段描述展示在 RustDoc
struct Person {
    #[graphql(description="这段描述展示在 GraphQL")]
    /// 这段描述展示在 RustDoc
    name: String,
    /// 这段描述在 RustDoc 和 GraphQL 中都展示
    age: i32,
}

# fn main() {}
```

## 关系

如下情形，只能使用自定义派生属性：

- 注解类型是`结构体（struct）`,
- 结构体的字段符合以下情形——
  - 简单类型（`i32`, `f64`, `bool`, `String`, `juniper::ID`）；或者
  - 有效的自定义 GraphQL 类型，如使用此属性标记了其他结构体字段；或者
  - 容器/引用包含以上情形之一，如 `Vec<T>`、`Box<T>`、`Option<T>`。

让我们看看这对于对象之间的构建关系意味着什么：

```rust
#[derive(juniper::GraphQLObject)]
struct Person {
    name: String,
    age: i32,
}

#[derive(juniper::GraphQLObject)]
struct House {
    address: Option<String>, // 转换为字符串（可空）
    inhabitants: Vec<Person>, // 转换为 [Person!]!
}

# fn main() {}
```

因为 `Person` 是一个有效的 GraphQL 类型，所以可以在另一个结构体中使用 `Vec<Person>`，它将自动转换为 `非空 Person 对象` 的列表。

## 字段重命名

默认地，结构体字段由 Rust 标准命名约定`蛇形命名法（snake_case）`被转换为 GraphQL 约定的`驼峰命名法（snake_case）`：

```rust
#[derive(juniper::GraphQLObject)]
struct Person {
    first_name: String, // GraphQL 模式中将被暴露为 firstName
    last_name: String, // GraphQL 模式中将被暴露为 lastName
}

# fn main() {}
```

可以在某个结构体字段上使用 `graphql` 属性指定名称：

```rust
#[derive(juniper::GraphQLObject)]
struct Person {
    name: String,
    age: i32,
    #[graphql(name="websiteURL")]
    website_url: Option<String>, // GraphQL 模式中将被暴露为 websiteURL
}

# fn main() {}
```

## 字段弃用

要弃用字段，可使用 `graphql` 属性指定弃用原因：

```rust
#[derive(juniper::GraphQLObject)]
struct Person {
    name: String,
    age: i32,
    #[graphql(deprecated = "请使用 name 字段代替")]
    first_name: String,
}

# fn main() {}
```

当然，`名称（name）`、`描述(description)`和`deprecation（弃用）`参数可以组合使用。不过 GraphQL 规范中的一些限制依然存在，`deprecation（弃用）`参数只能用于对象字段和枚举值。

## 字段忽略

默认地，`GraphQLObject` 中的所有字段都包含在生成的 GraphQL 类型中。若要不包含特定字段，请使用注解 `#[graphql(skip)]`：

```rust
#[derive(juniper::GraphQLObject)]
struct Person {
    name: String,
    age: i32,
    #[graphql(skip)]
    # #[allow(dead_code)]
    password_hash: String, // 此字段不能从 GraphQL 查询或修改
}

# fn main() {}
```
