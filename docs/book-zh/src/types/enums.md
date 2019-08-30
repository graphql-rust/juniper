# 枚举

> [types/enums.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/types/enums.md)
> <br />
> commit a75396846d9f8930d1e07e972a91ff59308e77cf

GraphQL 中的枚举是聚在一起表示一组值的字符串常量。通过自定义派生属性，可以将简单的 Rust 枚举转换为 GraphQL 枚举：

```rust
#[derive(juniper::GraphQLEnum)]
enum Episode {
    NewHope,
    Empire,
    Jedi,
}

# fn main() {}
```

Juniper 会将枚举变量转换为大写，因此这些变量对应的字符串值分别是 `NEWHOPE`、`EMPIRE`、`JEDI`。如果你想要重写，可使用 `graphql` 属性，如同我们在[对象定义](objects/defining_objects.md)学习的那样：

```rust
#[derive(juniper::GraphQLEnum)]
enum Episode {
    #[graphql(name="NEW_HOPE")]
    NewHope,
    Empire,
    Jedi,
}

# fn main() {}
```

## 文档化和弃用

就像定义对象一样，类型自身可以重命名和文档化。同样地，枚举变量可以重命名、文档化，以及弃用：

```rust
#[derive(juniper::GraphQLEnum)]
#[graphql(name="Episode", description="星球大战4：新希望")]
enum StarWarsEpisode {
    #[graphql(deprecated="对此，我们不做讨论")]
    ThePhantomMenace,

    #[graphql(name="NEW_HOPE")]
    NewHope,

    #[graphql(description="最好的一部")]
    Empire,
    Jedi,
}

# fn main() {}
```
