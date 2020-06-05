# Enums

Enums in GraphQL are string constants grouped together to represent a set of
possible values. Simple Rust enums can be converted to GraphQL enums by using a
custom derive attribute:

```rust
#[derive(juniper::GraphQLEnum)]
enum Episode {
    NewHope,
    Empire,
    Jedi,
}

# fn main() {}
```

Juniper converts all enum variants to uppercase, so the corresponding string
values for these variants are `NEWHOPE`, `EMPIRE`, and `JEDI`, respectively. If
you want to override this, you can use the `graphql` attribute, similar to how
it works when [defining objects](objects/defining_objects.md):

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

## Documentation and deprecation

Just like when defining objects, the type itself can be renamed and documented,
while individual enum variants can be renamed, documented, and deprecated:

```rust
#[derive(juniper::GraphQLEnum)]
#[graphql(name="Episode", description="An episode of Star Wars")]
enum StarWarsEpisode {
    #[graphql(deprecated="We don't really talk about this one")]
    ThePhantomMenace,

    #[graphql(name="NEW_HOPE")]
    NewHope,

    #[graphql(description="Arguably the best one in the trilogy")]
    Empire,
    Jedi,
}

# fn main() {}
```

## Supported Macro Attributes (Derive)

| Name of Attribute | Container Support | Field Support    |
|-------------------|:-----------------:|:----------------:|
| context           | ✔                 | ?                |
| deprecated        | ✔                 | ✔                |
| description       | ✔                 | ✔                |
| interfaces        | ?                 | ✘                |
| name              | ✔                 | ✔                |
| noasync           | ✔                 | ?                |
| scalar            | ✘                 | ?                |
| skip              | ?                 | ✘                |
| ✔: supported      | ✘: not supported  | ?: not available |
