# Input objects

Input objects are complex data structures that can be used as arguments to
GraphQL fields. In Juniper, you can define input objects using a custom derive
attribute, similar to simple objects and enums:

```rust
#[derive(juniper::GraphQLInputObject)]
struct Coordinate {
    latitude: f64,
    longitude: f64
}

struct Root;
# #[derive(juniper::GraphQLObject)] struct User { name: String }

#[juniper::graphql_object]
impl Root {
    fn users_at_location(coordinate: Coordinate, radius: f64) -> Vec<User> {
        // Send coordinate to database
        // ...
# unimplemented!()
    }
}

# fn main() {}
```

## Documentation and renaming

Just like the [other](objects/defining_objects.md) [derives](enums.md), you can rename
and add documentation to both the type and the fields:

```rust
#[derive(juniper::GraphQLInputObject)]
#[graphql(name="Coordinate", description="A position on the globe")]
struct WorldCoordinate {
    #[graphql(name="lat", description="The latitude")]
    latitude: f64,

    #[graphql(name="long", description="The longitude")]
    longitude: f64
}

struct Root;
# #[derive(juniper::GraphQLObject)] struct User { name: String }

#[juniper::graphql_object]
impl Root {
    fn users_at_location(coordinate: WorldCoordinate, radius: f64) -> Vec<User> {
        // Send coordinate to database
        // ...
# unimplemented!()
    }
}

# fn main() {}
```
