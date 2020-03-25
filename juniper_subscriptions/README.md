# juniper_subscriptions

This repository contains [SubscriptionCoordinator][SubscriptionCoordinator] and 
[SubscriptionConnection][SubscriptionConnection] implementations for 
[Juniper][Juniper], a [GraphQL][GraphQL] library for Rust.

## Documentation

For this crate's documentation, check out [API documentation][documentation].

For `SubscriptionCoordinator` and `SubscriptionConnection` documentation, check 
out [Juniper][Juniper]. 

## Examples

Check [examples/warp_subscriptions][example] for example code of a working 
[warp][warp] server with GraphQL subscription handlers.

## Links

* [Juniper][Juniper]
* [API Reference][documentation]
* [warp][warp]

## License

This project is under the BSD-2 license.

Check the LICENSE file for details.

[warp]: https://github.com/seanmonstar/warp
[Juniper]: https://github.com/graphql-rust/juniper
[SubscriptionCoordinator]: https://docs.rs/juniper/latest/juniper/trait.SubscriptionCoordinator.html
[SubscriptionConnection]: https://docs.rs/juniper/latest/juniper/trait.SubscriptionConnection.html
[GraphQL]: http://graphql.org
[documentation]: https://docs.rs/juniper_subscriptions
[example]: https://github.com/graphql-rust/juniper/blob/master/examples/warp_subscriptions/src/main.rs
