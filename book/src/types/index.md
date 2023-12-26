Type system
===========

Most of the work in working with [Juniper] consists of mapping the [GraphQL type system][0] to the [Rust] types our application uses.

[Juniper] provides some convenient abstractions making this process as painless as possible.

Find out more in the individual chapters below:
- [Objects](objects/index.md)
    - [Complex fields](objects/complex_fields.md)
    - [Context](objects/Context.md)
    - [Error handling](objects/error/index.md)
        - [Field errors](objects/error/field.md)
        - [Schema errors](objects/error/schema.md)
    - [Generics](objects/generics.md)
- [Interfaces](interfaces.md)
- [Unions](unions.md)
- [Enums](enums.md)
- [Input objects](input_objects.md)
- [Scalars](scalars.md)




[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org

[0]: https://spec.graphql.org/October2021#sec-Type-System
