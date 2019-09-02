# 对象和泛型

> [advanced/objects_and_generics.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/advanced/objects_and_generics.md)
> <br />
> commit 29025e6cae4a249fa56017dcf16b95ee4e89363e

GraphQL 和 Rust 的另一个差异是泛型。Rust 中，几乎任何类型都可以是泛型的——即接受类型参数。GraphQL 中，只有两种泛型类型：列表（lists）和非空值（non-nullables）。

这对从 Rust 向 GraphQL 暴露内容造成了限制：不能暴露泛型结构，而必须绑定类型参数。例如，你不能将 `Result<T, E>` 转换为 GraphQL 类型，但你**能够**将 `Result<User, String>` 转换为 GraphQL 类型。

让我们对[非结构化对象](non_struct_objects.md)中的示例做一些细小紧凑的改动，来进行泛型实现：

```rust
# #[derive(juniper::GraphQLObject)] struct User { name: String }
# #[derive(juniper::GraphQLObject)] struct ForumPost { title: String }

#[derive(juniper::GraphQLObject)]
struct ValidationError {
    field: String,
    message: String,
}

# #[allow(dead_code)]
struct MutationResult<T>(Result<T, Vec<ValidationError>>);

#[juniper::object(
    name = "UserResult",
)]
impl MutationResult<User> {
    fn user(&self) -> Option<&User> {
        self.0.as_ref().ok()
    }

    fn error(&self) -> Option<&Vec<ValidationError>> {
        self.0.as_ref().err()
    }
}

#[juniper::object(
    name = "ForumPostResult",
)]
impl MutationResult<ForumPost> {
    fn forum_post(&self) -> Option<&ForumPost> {
        self.0.as_ref().ok()
    }

    fn error(&self) -> Option<&Vec<ValidationError>> {
        self.0.as_ref().err()
    }
}

# fn main() {}
```

我们对 `Result` 做了包装，并暴露 `Result<T, E>` 的具体实例为不同的 GraphQL 对象。我们需要包装的原因是 Rust 具有派生特性的规则——本例中，`Result` 和 Juniper 的内部 GraphQL 特性都来自第三方。

因为我们使用泛型，所以还需要为实例化的类型指定一个名字。即使 Juniper **能够**找出名字，`MutationResult<User>` 也不是有效的 GraphQL 类型名。
