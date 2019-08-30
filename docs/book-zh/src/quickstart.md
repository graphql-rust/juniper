# 快速入门

> [quickstart.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/quickstart.md)
> <br />
> commit 29025e6cae4a249fa56017dcf16b95ee4e89363e

简要介绍 Juniper 中的概念。

## 安装

!文件名 Cargo.toml

```toml
[dependencies]
juniper = "^0.13.1"
```

## 模式示例

要将 Rust 语言的 `enums` 和 `structs` 暴露为 GraphQL，仅需向其增加一个自定义`派生属性`。Juniper 支持将 Rust 语言基本类型轻而易举地映射到 GraphQL 特性，诸如：`Option<T>`、`Vec<T>`、`Box<T>`、`String`、`f64` 和 `i32`、`引用`和`切片（slice）`.

对于更高级的映射，Juniper 提供了多种`宏（macro）`来将 Rust 类型映射到 GraphQL 模式。[过程宏对象][jp_obj_macro]是最重要的宏对象之一，其用于声明解析器对象，你将使用解析器对象来 `查询（Query）` 和 `变更（Mutation）` 根（roots）。

```rust
use juniper::{FieldResult};

# struct DatabasePool;
# impl DatabasePool {
#     fn get_connection(&self) -> FieldResult<DatabasePool> { Ok(DatabasePool) }
#     fn find_human(&self, _id: &str) -> FieldResult<Human> { Err("")? }
#     fn insert_human(&self, _human: &NewHuman) -> FieldResult<Human> { Err("")? }
# }

#[derive(juniper::GraphQLEnum)]
enum Episode {
    NewHope,
    Empire,
    Jedi,
}

#[derive(juniper::GraphQLObject)]
#[graphql(description="星球大战中的类人生物")]
struct Human {
    id: String,
    name: String,
    appears_in: Vec<Episode>,
    home_planet: String,
}

// 另一个用于映射 GraphQL 输入对象的自定义派生。

#[derive(juniper::GraphQLInputObject)]
#[graphql(description="星球大战中的类人生物")]
struct NewHuman {
    name: String,
    appears_in: Vec<Episode>,
    home_planet: String,
}

// 使用宏对象创建带有解析器的根查询和根变更。
// 对象可以拥有类似数据库池一样的允许访问共享状态的上下文。

struct Context {
    // 这里使用真实数据池
    pool: DatabasePool,
}

// 要让 Juniper 使用上下文，必须实现标注特性（trait）
impl juniper::Context for Context {}

struct Query;

#[juniper::object(
    // 指定对象的上下文类型。
    // 需要访问上下文的每种类型都需如此。
    Context = Context,
)]
impl Query {

    fn apiVersion() -> &str {
        "1.0"
    }

    // 解析器的参数可以是简单类型，也可以是输入对象。
    // 为了访问上下文，我们指定了一个引用上下文类型的参数。
    // Juniper 会自动注入正确的上下文。
    fn human(context: &Context, id: String) -> FieldResult<Human> {
        // 获取数据库连接
        let connection = context.pool.get_connection()?;
        // 执行查询
        // 注意 `?` 的用法，进行错误传播。
        // 译者注：上一章“特点”中提到，Juniper 默认构建非空类型
        let human = connection.find_human(&id)?;
        // 返回结果集
        Ok(human)
    }
}

// 下面对变更类型做同样的事情

struct Mutation;

#[juniper::object(
    Context = Context,
)]
impl Mutation {

    fn createHuman(context: &Context, new_human: NewHuman) -> FieldResult<Human> {
        let db = executor.context().pool.get_connection()?;
        let human: Human = db.insert_human(&new_human)?;
        Ok(human)
    }
}

// 根模式由查询和变更组成，故查询请求可以执行于 RootNode。
type Schema = juniper::RootNode<'static, Query, Mutation>;

# fn main() {
#   let _ = Schema::new(Query, Mutation{});
# }
```

现在，我们有了一个非常简单，但模式功能齐全的 GraphQL服务器。

要让此模式在服务器端起作用，查阅各类[服务器集成](./servers/index.md)指南。

也可以直接调用执行器（executor）来获取查询结果集：

## 执行器（executor）

可以直接调用 `juniper::execute` 来运行 GraphQL 查询：

```rust
# // 由于宏（macro）不可访问，如下代码仅 Rust-2018 版需要
# #[macro_use] extern crate juniper;
use juniper::{FieldResult, Variables, EmptyMutation};


#[derive(juniper::GraphQLEnum, Clone, Copy)]
enum Episode {
    NewHope,
    Empire,
    Jedi,
}

// 上下文（context）数据
struct Ctx(Episode);

impl juniper::Context for Ctx {}

struct Query;

#[juniper::object(
    Context = Ctx,
)]
impl Query {
    fn favoriteEpisode(context: &Ctx) -> FieldResult<Episode> {
        Ok(context.0)
    }
}


// 根模式由查询和变更组成，故查询请求可以执行于 RootNode。
type Schema = juniper::RootNode<'static, Query, EmptyMutation<Ctx>>;

fn main() {
    // 创建上下文对象
    let ctx = Ctx(Episode::NewHope);

    // 运行执行器
    let (res, _errors) = juniper::execute(
        "query { favoriteEpisode }",
        None,
        &Schema::new(Query, EmptyMutation::new()),
        &Variables::new(),
        &ctx,
    ).unwrap();

    // 确保查询结果值匹配
    assert_eq!(
        res,
        graphql_value!({
            "favoriteEpisode": "NEW_HOPE",
        })
    );
}
```

[hyper]: servers/hyper.md
[warp]: servers/warp.md
[rocket]: servers/rocket.md
[iron]: servers/iron.md
[tutorial]: ./tutorial.html
[jp_obj_macro]: https://docs.rs/juniper/latest/juniper/macro.object.html
