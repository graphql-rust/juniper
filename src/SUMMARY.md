# Summary

- [Introduction](README.md)
- [Quickstart](quickstart.md)

## Type System

- [Defining objects](types/objects/defining_objects.md)
  - [Complex fields](types/objects/complex_fields.md)
  - [Using contexts](types/objects/using_contexts.md)
  - [Error handling](types/objects/error_handling.md)
- Other types
  - [Enums](types/enums.md)
  - [Interfaces](types/interfaces.md)
  - [Input objects](types/input_objects.md)
  - [Scalars](types/scalars.md)
  - [Unions](types/unions.md)

## Schema

- [Schemas and mutations](schema/schemas_and_mutations.md)

## Adding a server

- Integrations by Juniper
  - [Hyper](servers/hyper.md)
  - [Warp](servers/warp.md)
  - [Rocket](servers/rocket.md)
  - [Iron](servers/iron.md)
- Integrations by others
  - [Actix-Web](https://github.com/actix/examples/tree/master/juniper)
  - [Finchers](https://github.com/finchers-rs/finchers-juniper)
  - [Tsukuyomi](https://github.com/tsukuyomi-rs/tsukuyomi/tree/master/examples/juniper)

## Advanced Topics

- [Non-struct objects](advanced/non_struct_objects.md)
- [Objects and generics](advanced/objects_and_generics.md)
- [Context switching]
- [Dynamic type system]
- [Multiple operations per request](advanced/multiple_ops_per_request.md)
