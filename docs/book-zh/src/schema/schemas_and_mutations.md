# 模式（Schemas）

> [schema/schemas_and_mutations.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/schema/schemas_and_mutations.md)
> <br />
> commit 29025e6cae4a249fa56017dcf16b95ee4e89363e

模式由两种类型组成：查询对象和变更对象（Juniper 还不支持订阅），分别定义了字段查询和模式变更。

查询和变更都是常规的 GraphQL 对象，其定义类似于 Juniper 中的其它对象。不过变更对象是可选的，因为模式可以是只读性的。

Juniper 中，`RootNode` 类型表示一个模式。通常不需要你自己创建此对象：请参阅 [Iron](../servers/iron.md) 和 [Rocket](../servers/rocket.md) 的框架集成，了解模式与框架处理程序是如何被一起创建的。

模式首次创建时，Juniper 将遍历整个对象图，并注册所有类型。这意味着，如果定义了 GraphQL 对象但从未引用，那么此对象就不会暴露在模式中。

## 查询根（query root）

查询根（query root）也是 GraphQL 对象。其定义类似于 Juniper 中的其它对象，通常使用`过程宏对象`定义查询根：

```rust
# use juniper::FieldResult;
# #[derive(juniper::GraphQLObject)] struct User { name: String }
struct Root;

#[juniper::object]
impl Root {
    fn userWithUsername(username: String) -> FieldResult<Option<User>> {
        // 在数据库查找用户
# unimplemented!()
    }
}

# fn main() { }
```

## 变更

变更同样是 GraphQL 对象。变更是字段发生一些改变，如更新数据库。

```rust
# use juniper::FieldResult;
# #[derive(juniper::GraphQLObject)] struct User { name: String }
struct Mutations;

#[juniper::object]
impl Mutations {
    fn signUpUser(name: String, email: String) -> FieldResult<User> {
        // 验证输入并存储数据
# unimplemented!()
    }
}

# fn main() { }
```
