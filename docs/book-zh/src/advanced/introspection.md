# 内省

> [advanced/introspection.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/advanced/introspection.md)
> <br />
> commit 29025e6cae4a249fa56017dcf16b95ee4e89363e

GraphQL 内建了一个特殊的顶级字段 `__schema`，查询此字段允许在运行时[内省模式](https://graphql.org/learn/introspection)，以查看 GraphQL 服务器支持的查询（queries）和变更（mutations）。

内省查询是 GraphQL 常规查询，因此 Juniper 原生支持。例如，要获得支持类型的所有名称，可以对 Juniper 执行以下查询：

```graphql
{
  __schema {
    types {
      name
    }
  }
}
```

## 模式内省输出为 JSON

GraphQL 生态中，许多客户端库和工具都需要完整的服务器模式描述。通常，描述是 JSON 格式的，被称为 `schema.json`。可以通过特别设计的内省查询生成模式的完整描述。

Juniper 提供函数来内省整个模式，将结果转换为 JSON，以便与 [graphql-client](https://github.com/graphql-rust/graphql-client) 之类的工具和库一起使用：

```rust
use juniper::{EmptyMutation, FieldResult, IntrospectionFormat};

// 定义模式（schema）

#[derive(juniper::GraphQLObject)]
struct Example {
  id: String,
}

struct Context;
impl juniper::Context for Context {}

struct Query;

#[juniper::object(
  Context = Context,
)]
impl Query {
   fn example(id: String) -> FieldResult<Example> {
       unimplemented!()
   }
}

type Schema = juniper::RootNode<'static, Query, EmptyMutation<Context>>;

fn main() {
    // 创建上下文对象
    let ctx = Context{};

    // 运行内建内省查询
    let (res, _errors) = juniper::introspect(
        &Schema::new(Query, EmptyMutation::new()),
        &ctx,
        IntrospectionFormat::default(),
    ).unwrap();

    // 转换内省结果为 JSON
    let json_result = serde_json::to_string_pretty(&res);
    assert!(json_result.is_ok());
}
```
