# juniper_iron

[![Build Status](https://travis-ci.org/graphql-rust/juniper_iron.svg?branch=master)](https://travis-ci.org/graphql-rust/juniper_iron)
[![Build status](https://ci.appveyor.com/api/projects/status/rqguvfkl9m0g7hum?svg=true)](https://ci.appveyor.com/project/theduke/juniper-iron)
[![Crates.io](https://img.shields.io/crates/v/juniper_iron.svg?maxAge=2592000)](https://crates.io/crates/juniper_iron)
[![Gitter chat](https://badges.gitter.im/juniper-graphql/gitter.png)](https://gitter.im/juniper-graphql)

This repository contains the [Iron][Iron] web framework integration for [Juniper][Juniper], a [GraphQL][GraphQL] 
implementation for Rust.

## Documentation

Once the crate is published, documentation will be on [docs.rs][documentation].

For now, please consult the documentation comments [here](https://github.com/graphql-rust/juniper_iron/blob/master/src/lib.rs).

## Examples

Check [examples/iron_server.rs][example] for example code of a working Iron server with GraphQL handlers.

## License

This project is under the BSD-2 license.

Check the LICENSE file for details.

[Iron]: https://github.com/iron/iron
[Juniper]: https://github.com/graphql-rust/juniper
[GraphQL]: http://graphql.org
[documentation]: https://docs.rs/juniper_iron
[example]: https://github.com/graphql-rust/juniper_iron/blob/master/examples/iron_server.rs
