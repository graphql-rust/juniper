# 标量

> [types/scalars.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/types/scalars.md)
> <br />
> commit 97e1005178b4bd4d96d85ffb7c82cd36b66380b3

GraphQL 查询中，标量是叶子节点的基本类型：ID、数字、字符串和布尔值。可以为其它基本值创建自定义标量，但这需要与你正在构建 API 的客户端库进行协调。

由于任何值都是以 JSON 格式传递，所以可用的数据类型也受到了限制。

自定义标量有两种方式。

* 对于只封装基本类型的简单标量，可以使用自定义派生对象的新类型（newtype）方式。
* 对于有自定义验证的高级用例标量，可以使用 `graphql_scalar!` 宏。

## 内建标量

Juniper 内建支持的标量有：

* `i32` 表示 `Int`：有符号 32 位整数；
* `f64` 表示 `Float`：有符号双精度浮点值；
* `String` 和 `&str` 表示 `String`：UTF‐8 字符序列；
* `bool` 表示 `Boolean`：true 或者 false；
* `juniper::ID` 表示 `ID`：此类型在[规范](http://facebook.github.io/graphql/#sec-ID)中被定义为序列化字符串类型，但可以从字符串和整数解析。

**第三方类型**：

Juniper 内建支持一些来自常用第三方库的附加类型，此特性支持默认开启。

* uuid::Uuid
* chrono::DateTime
* url::Url

## 新类型（newtype）方式

通常情况下，你可能仅需要只包装已有类型的自定义标量。

这可以通过新类型（newtype）方式和自定义派生来实现，类似于 serde 的方式 `#[serde(transparent)]`。

```rust
#[derive(juniper::GraphQLScalarValue)]
pub struct UserId(i32);

#[derive(juniper::GraphQLObject)]
struct User {
    id: UserId,
}

# fn main() {}
```

就这样简单，然后就可以在你的模式中使用 `UserId`。

宏（macro）同样允许很多定制：

```rust
/// 可以使用文档注释指定描述。
#[derive(juniper::GraphQLScalarValue)]
#[graphql(
    transparent,
    // 可以重写 GraphQL 类型 name 属性
    name = "MyUserId",
    // 指定自定义描述
    // 属性中的描述将覆盖文档注释
    description = "这是自定义用户描述",
)]
pub struct UserId(i32);

# fn main() {}
```

## 自定义标量

对于需要自定义解析或验证的复杂情况，可以使用 `graphql_scalar!` 宏。

通常，将自定义标量表示为字符串。

下面的例子中，为自定义 `Date` 类型实现自定义标量。

注意：Juniper 通过 `chrono` 特性内建支持 `chrono::DateTime` 类型，为此目的此特性默认开启。

下面的例子仅为举例说明。

**注意**：本例假定 `Date` 类型实现 `std::fmt::Display` 和 `std::str::FromStr`。

```rust
# mod date { 
#    pub struct Date; 
#    impl std::str::FromStr for Date{ 
#        type Err = String; fn from_str(_value: &str) -> Result<Self, Self::Err> { unimplemented!() }
#    }
#    // 定义如何将日期表示为字符串
#    impl std::fmt::Display for Date {
#        fn fmt(&self, _f: &mut std::fmt::Formatter) -> std::fmt::Result {
#            unimplemented!()
#        }
#    }
# }

use juniper::{Value, ParseScalarResult, ParseScalarValue};
use date::Date;

juniper::graphql_scalar!(Date where Scalar = <S> {
    description: "Date"

    // 定义如何将自定义标量转换为基本类型
    resolve(&self) -> Value {
        Value::scalar(self.to_string())
    }

    // 定义如何将基本类型解析为自定义标量
    from_input_value(v: &InputValue) -> Option<Date> {
        v.as_scalar_value::<String>()
         .and_then(|s| s.parse().ok())
    }

    // 定义如何解析字符串值
    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
});

# fn main() {}
```
