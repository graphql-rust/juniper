# 复杂字段

> [types/objects/complex_fields.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/types/objects/complex_fields.md)
> <br />
> commit cff6036206da12f9a4cbddb869569e9a977fa2ef

如果你有一个不能直接映射到 GraphQL 的结构体（struct），其中包含计算字段或循环结构，那么你必须使用一个更强大的工具：`过程宏对象`。过程宏允许你在 Rust `impl` 块中为类型定义 GraphQL 对象字段。让我们继续上一章的示例，学习如何使用宏定义 `Person`：

```rust

struct Person {
    name: String,
    age: i32,
}

#[juniper::object]
impl Person {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn age(&self) -> i32 {
        self.age
    }
}

# fn main() { }
```

虽然上述示例代码有点冗长，但它允许你在字段解析器中编写任何类型的函数。同时，使用上述示例语法，字段可以接受参数：

```rust
#[derive(juniper::GraphQLObject)]
struct Person {
    name: String,
    age: i32,
}

struct House {
    inhabitants: Vec<Person>,
}

#[juniper::object]
impl House {
    // 创建字段 inhabitantWithName(name), 返回非空 person
    fn inhabitant_with_name(&self, name: String) -> Option<&Person> {
        self.inhabitants.iter().find(|p| p.name == name)
    }
}

# fn main() {}
```

要访问诸如数据库连接或身份验证信息之类的全局数据，需要使用 _上下文（context）_。更多关于 _上下文（context）_ 的信息，将在下一章介绍：[上下文](using_contexts.md)。

## 描述、重命名，以及弃用

与派生属性一样，字段名将从命名约定 `snake_case` 转换为 `camelCase`。若需重写转换，可以简单地重命名字段。此外，可以使用别名更换类型名称：

```rust

struct Person {
}

/// Rust 文档注释用作 GraphQL 描述。
#[juniper::object(
    // 使用 name 属性，可以更改 GraphQL 类型的公开名称。
    name = "PersonObject",
    // 可以在此处指定 GraphQL 描述，这将覆盖 Rust 文档注释。
    description = "...",
)]
impl Person {

    /// 字段上的文档注释被用作 GraphQL 描述
    #[graphql(
        // 或者指定 GraphQL 描述
        description = "...",
    )]
    fn doc_comment(&self) -> &str {
        ""
    }

    // 如果需要，字段也可以使用 name 属性来重命名
    #[graphql(
        name = "myCustomFieldName",
    )]
    fn renamed_field() -> bool {
        true
    }

    // 如期望的那样，弃用也有效。
    // 即可以接受标准的 Rust 语法，也可以接受自定义属性。
    #[deprecated(note = "...")]
    fn deprecated_standard() -> bool {
        false
    }

    #[graphql(deprecated = "...")]
    fn deprecated_graphql() -> bool {
        true
    }
}

# fn main() { }
```

## 自定义参数

方法的字段参数也是可以定制的，可以指定自定义描述和默认值。

**注意**：此语法目前有点别扭。一旦实现了 [Rust RFC 2565](https://github.com/rust-lang/rust/issues/60406)，将会变得好用。

```rust

struct Person {}

#[juniper::object]
impl Person {
    #[graphql(
        arguments(
            arg1(
                // 设置默认值，如果字段没有被赋值，默认值将被使用。
                // 默认值可以是任何有效的 Rust 表达式，甚至调用函数等。
                default = true,
                // 设定 GraphQL 描述
                description = "第一个参数..."
            ),
            arg2(
                default = 0,
            )
        )
    )]
    fn field1(&self, arg1: bool, arg2: i32) -> String {
        format!("{} {}", arg1, arg2)
    }
}

# fn main() { }
```

## 更多功能

GraphQL 字段提供了比 Rust 标准方法提供了更多的功能：

* 字段的描述和弃用消息；
* 参数默认值；
* 参数描述

更多的特性在[参考文档](https://docs.rs/juniper/latest/juniper/macro.object.html)中有详细的描述。
