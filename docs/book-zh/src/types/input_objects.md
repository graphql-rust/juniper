# 输入对象

> [types/input_objects.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/types/input_objects.md)
> <br />
> commit 29025e6cae4a249fa56017dcf16b95ee4e89363e

输入对象是复杂的数据结构，可以用作 GraphQL 字段的参数。Juniper 中，可以使用自定义派生属性来定义输入对象，类似于定义简单对象、枚举：

```rust
#[derive(juniper::GraphQLInputObject)]
struct Coordinate {
    latitude: f64,
    longitude: f64
}

struct Root;
# #[derive(juniper::GraphQLObject)] struct User { name: String }

#[juniper::object]
impl Root {
    fn users_at_location(coordinate: Coordinate, radius: f64) -> Vec<User> {
        // 将坐标写入数据库
        // ...
# unimplemented!()
    }
}

# fn main() {}
```

## 文档化和重命名

类似于[定义对象](objects/defining_objects.md)、[派生枚举对象](enums.md)，对类型和字段，既可以重命名，也可以添加文档：

```rust
#[derive(juniper::GraphQLInputObject)]
#[graphql(name="Coordinate", description="地球某一处")]
struct WorldCoordinate {
    #[graphql(name="lat", description="维度")]
    latitude: f64,

    #[graphql(name="long", description="精度")]
    longitude: f64
}

struct Root;
# #[derive(juniper::GraphQLObject)] struct User { name: String }

#[juniper::object]
impl Root {
    fn users_at_location(coordinate: WorldCoordinate, radius: f64) -> Vec<User> {
        // 将坐标写入数据库
        // ...
# unimplemented!()
    }
}

# fn main() {}
```
