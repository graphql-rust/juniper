Change log
==========

## [Unreleased]

The repository was restructured to a multi crate workspace to enable several new features like custom_derive and an extracted parser.

### New features

* New juniper_codegen crate which provides custom derives: 
  * `#[derive(GraphQLInputObject)]`
  * `#[derive(GraphQLEnum)]`

## [0.8.1] – 2017-06-15

Tiny release to fix broken crate metadata on crates.io.

## [0.8.0] – 2017-06-15

## Breaking changes

* To better comply with the specification, and to avoid weird bugs with very
  large positive or negative integers, support for `i64` has been completely
  dropped and replaced with `i32`. `i64` is no longer a valid GraphQL type in
  Juniper, and `InputValue`/`Value` can only represent 32 bit integers.

  If an incoming integer is out of range for a 32 bit signed integer type, an
  error will be returned to the client.
  ([#52](https://github.com/mhallin/juniper/issues/52),
  [#49](https://github.com/mhallin/juniper/issues/49))

* Serde has been updated to 1.0. If your application depends on an older
  version, you will need to first update your application before you can upgrade
  to a more recent Juniper. ([#43](https://github.com/mhallin/juniper/pull/43))

* `rustc_serialize` support has been dropped since this library is now
  deprecated. ([#51](https://github.com/mhallin/juniper/pull/51))

## New features

* A new `rocket-handlers` feature now includes some tools to use the
  [Rocket](https://rocket.rs) framework. [A simple
  example](examples/rocket-server.rs) has been added to the examples folder.

## Bugfixes

* A panic in the parser has been replaced with a proper error
  ([#44](https://github.com/mhallin/juniper/pull/44))

## [0.7.0] – 2017-02-26

### Breaking changes

* The `iron-handlers` feature now depends on Iron 0.5
  ([#30](https://github.com/mhallin/juniper/pull/30)). Because of
  this, support for Rust 1.12 has been dropped. It might still work if
  you're not using the Iron integrations feature, however.

### New features

* Input objects defined by the `graphql_input_object!` can now be used
  as default values to field arguments and other input object fields.


## [0.6.3] – 2017-02-19

### New features

* Add support for default values on input object fields
  ([#28](https://github.com/mhallin/juniper/issues/28))

## [0.6.2] – 2017-02-05

### New features

* The `null` literal is now supported in the GraphQL
  language. ([#26](https://github.com/mhallin/juniper/pull/26))
* Rustc-serialize is now optional, but enabled by default. If you
  _only_ want Serde support, include Juniper without default features
  and enable
  Serde. ([#12](https://github.com/mhallin/juniper/pull/12))
* The built-in `ID` type now has a public constructor and derives a
  few traits (`Clone`, `Debug`, `Eq`, `PartialEq`, `From<String>`,
  `Deref<Target=str>`). ([#19](https://github.com/mhallin/juniper/pull/19))
* Juniper is now built and tested against all Rust compilers since
  version 1.12.1.

### Minor breaking change

* Serde has been updated to
  0.9. ([#25](https://github.com/mhallin/juniper/pull/25))

### Bugfixes

* The built-in GraphiQL handler had a bug in variable serialization.
  ([#16](https://github.com/mhallin/juniper/pull/16))
* The example should now build and run without problems on
  Windows. ([#15](https://github.com/mhallin/juniper/pull/15))
* Object types now properly implement
  `__typename`. ([#22](https://github.com/mhallin/juniper/pull/22))
* String variables are now properly parsed into GraphQL
  enums. ([#17](https://github.com/mhallin/juniper/pull/17))

## [0.6.1] – 2017-01-06

### New features

* Optional Serde support
  ([#8](https://github.com/mhallin/juniper/pull/8))

### Improvements

* The `graphql_input_object!` macro can now be used to define input
  objects as public Rust structs.
* GraphiQL in the Iron GraphiQL handler has been updated to 0.8.1
  (#[#11](https://github.com/mhallin/juniper/pull/11))

### Minor breaking changes

Some undocumented but public APIs were changed.

* `to_snake_case` correctly renamed to `to_camel_case`
  ([#9](https://github.com/mhallin/juniper/pull/9))
* JSON serialization of `GraphQLError` changed to be more consistent
  with how other values were serialized
  ([#10](https://github.com/mhallin/juniper/pull/10)).

## [0.6.0] – 2017-01-02

TL;DR: Many big changes in how context types work and how they
interact with the executor. Not too much to worry about if you're only
using the macros and not deriving `GraphQLType` directly.

### Breaking changes

* The `executor` argument in all resolver methods is now
  immutable. The executor instead uses interior mutability to store
  errors in a thread-safe manner.

  This change could open up for asynchronous or multi-threaded
  execution: you can today use something like rayon in your resolve
  methods to let child nodes be concurrently resolved.

  **How to fix:** All field resolvers that looked like `field
   name(&mut executor` now should say `field name(&executor`.

* The context type of `GraphQLType` is moved to an associated type;
  meaning it's no longer generic. This only affects people who
  implement the trait manually, _not_ macro users.

  This greatly simplifies a lot of code by ensuring that there only
  can be one `GraphQLType` implementation for any given Rust
  type. However, it has the downside that support for generic contexts
  previously used in scalars, has been removed. Instead, use the new
  context conversion features to accomplish the same task.

  **How to fix:** Instead of `impl GraphQLType<MyContext> for ...`,
   you use `impl GraphQLType for ... { type Context = MyContext;`.

* All context types must derive the `Context` marker trait. This is
  part of an overarching change to allow different types to use
  different contexts.

  **How to fix:** If you have written e.g. `graphql_object!(MyType:
   MyContext ...)` you will need to add `impl Context for MyContext
   {}`. Simple as that.

* `Registry` and all meta type structs now takes one lifetime
  parameter, which affects `GraphQLType`'s `meta` method. This only
  affects people who implement the trait manually.

  **How to fix:** Change the type signature of `meta()` to read `fn
    meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r>`.

* The type builder methods on `Registry` no longer return functions
  taking types or fields. Due to how the borrow checker works with
  expressions, you will have to split up the instantiation into two
  statements. This only affects people who implement the `GraphQLType`
  trait manually.

  **How to fix:** Change the contents of your `meta()` methods to
    something like this:

  ```rust
  fn meta<'r>(registry: &mut Registry<r>) -> MetaType<'r> {
      let fields = &[ /* your fields ... */ ];

      registry.build_object_type::<Self>(fields).into_meta()
  }

  ```


### Added

* Support for different contexts for different types. As GraphQL
  schemas tend to get large, narrowing down the context type to
  exactly what a given type needs is great for
  encapsulation. Similarly, letting different subsystems use different
  resources thorugh the context is also useful for the same reasons.

  Juniper supports two different methods of doing this, depending on
  your needs: if you have two contexts where one can be converted into
  the other _without any extra knowledge_, you can implement the new
  `FromContext` trait. This is useful if you have multiple crates or
  modules that all belong to the same GraphQL schema:

  ```rust

  struct TopContext {
    db: DatabaseConnection,
    session: WebSession,
    current_user: User,
  }

  struct ModuleOneContext {
    db: DatabaseConnection, // This module only requires a database connection
  }

  impl Context for TopContext {}
  impl Context for ModuleOneContext {}

  impl FromContext<TopContext> for ModuleOneContext {
    fn from(ctx: &TopContext) -> ModuleOneContext {
      ModuleOneContext {
        db: ctx.db.clone()
      }
    }
  }

  graphql_object!(Query: TopContext |&self| {
    field item(&executor) -> Item {
      executor.context().db.get_item()
    }
  });

  // The `Item` type uses another context type - conversion is automatic
  graphql_object!(Item: ModuleOneContext |&self| {
    // ...
  });

  ```

  The other way is to manually perform the conversion in a field
  resolver. This method is preferred when the child context needs
  extra knowledge than what exists in the parent context:

  ```rust

  // Each entity has its own context
  struct TopContext {
    entities: HashMap<i32, EntityContext>,
    db: DatabaseConnection,
  }

  struct EntityContext {
    // fields
  }

  impl Context for TopContext {}
  impl Context for EntityContext {}

  graphql_object!(Query: TopContext |&self| {
    // By returning a tuple (&Context, GraphQLType), you can tell the executor
    // to switch out the context for the returned value. You can wrap the
    // tuple in Option<>, FieldResult<>, FieldResult<Option<>>, or just return
    // the tuple without wrapping it.
    field entity(&executor, key: i32) -> Option<(&EntityContext, Entity)> {
      executor.context().entities.get(&key)
        .map(|ctx| (ctx, executor.context().db.get_entity(key)))
    }
  });

  graphql_object!(Entity: EntityContext |&self| {
    // ...
  });

  ```

### Improvements

* Parser and query execution has now reduced the allocation overhead
  by reusing as much as possible from the query source and meta type
  information.

## [0.5.3] – 2016-12-05

### Added

* `jtry!`: Helper macro to produce `FieldResult`s from regular
  `Result`s. Wherever you would be using `try!` in a regular function
  or method, you can use `jtry!` in a field resolver:

  ```rust
  graphql_object(MyType: Database |&self| {
    field count(&executor) -> FieldResult<i32> {
      let txn = jtry!(executor.context().transaction());

      let count = jtry!(txn.execute("SELECT COUNT(*) FROM user"));

      Ok(count[0][0])
    }
  });
  ```

### Changes

* Relax context type trait requirements for the iron handler: your
  contexts no longer have to be `Send + Sync`.

* `RootNode` is now `Send` and `Sync` if both the mutation and query
  types implement `Send` and `Sync`.

### Bugfixes

* `return` statements inside field resolvers no longer cause syntax
  errors.



## 0.5.2 – 2016-11-13

### Added

* Support for marking fields and enum values deprecated.
* `input_object!` helper macro

### Changes

* The included example server now uses the simple Star Wars schema
  used in query/introspection tests.

### Bugfixes

* The query validators - particularly ones concerned with validation
  of input data and variables - have been improved significantly. A
  large number of test cases have been added.

* Macro syntax stability has also been improved. All syntactical edge
  cases of the macros have gotten tests to verify their correctness.

[0.8.0]: https://github.com/mhallin/juniper/compare/0.8.0...0.8.1
[0.8.0]: https://github.com/mhallin/juniper/compare/0.7.0...0.8.0
[0.7.0]: https://github.com/mhallin/juniper/compare/0.6.3...0.7.0
[0.6.3]: https://github.com/mhallin/juniper/compare/0.6.2...0.6.3
[0.6.2]: https://github.com/mhallin/juniper/compare/0.6.1...0.6.2
[0.6.1]: https://github.com/mhallin/juniper/compare/0.6.0...0.6.1
[0.6.0]: https://github.com/mhallin/juniper/compare/0.5.3...0.6.0
[0.5.3]: https://github.com/mhallin/juniper/compare/0.5.2...0.5.3
