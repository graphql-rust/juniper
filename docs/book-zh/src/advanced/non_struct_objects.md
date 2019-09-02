# 非结构化对象

> [advanced/non_struct_objects.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/advanced/non_struct_objects.md)
> <br />
> commit 29025e6cae4a249fa56017dcf16b95ee4e89363e

到目前为止，我们只介绍了结构化数据到 GraphQL 对象的映射。实际上，任何 Rust 类型都可以映射到 GraphQL 对象中。本章中，我们将介绍枚举（enums），请注意枚举特性——枚举**不必**映射到 GraphQL 接口即可使用。

利用类似 `Result` 的枚举报告错误信息是常用的方式，例如报告变更时的验证错误：

```rust
# #[derive(juniper::GraphQLObject)] struct User { name: String }

#[derive(juniper::GraphQLObject)]
struct ValidationError {
    field: String,
    message: String,
}

# #[allow(dead_code)]
enum SignUpResult {
    Ok(User),
    Error(Vec<ValidationError>),
}

#[juniper::object]
impl SignUpResult {
    fn user(&self) -> Option<&User> {
        match *self {
            SignUpResult::Ok(ref user) => Some(user),
            SignUpResult::Error(_) => None,
        }
    }

    fn error(&self) -> Option<&Vec<ValidationError>> {
        match *self {
            SignUpResult::Ok(_) => None,
            SignUpResult::Error(ref errors) => Some(errors)
        }
    }
}

# fn main() {}
```

我们使用枚举来决定用户输入的数据是否有效，枚举可以作为返回结果。例如，注册（GraphQL 变更）的结果信息。

虽然这是关于如何使用结构化数据之外的 Rust 类型来描述 GraphQL 对象的示例，但同时也是一个关于如何为`“预期”错误`(如验证错误)实现错误处理的示例。对于如何在 GraphQL 中描述错误，Juniper 并没有严格的规则。对如何设计“硬”字段错误以及如何进行预期错误建模，GraphQL 的一位作者提出了一些意见：[客户端错误验证](https://github.com/facebook/graphql/issues/117#issuecomment-170180628)、[管理自定义用户错误的最佳方法](https://github.com/graphql/graphql-js/issues/560#issuecomment-259508214)。
