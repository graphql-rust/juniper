<img src="https://github.com/graphql-rust/juniper/raw/master/assets/logo/juniper-dark-word.png" alt="Juniper" width="500" />

> GraphQL server library for Rust

[![Build Status](https://dev.azure.com/graphql-rust/GraphQL%20Rust/_apis/build/status/graphql-rust.juniper)](https://dev.azure.com/graphql-rust/GraphQL%20Rust/_build/latest?definitionId=1)
[![codecov](https://codecov.io/gh/graphql-rust/juniper/branch/master/graph/badge.svg)](https://codecov.io/gh/graphql-rust/juniper)
[![Crates.io](https://img.shields.io/crates/v/juniper.svg?maxAge=2592000)](https://crates.io/crates/juniper)
[![Gitter chat](https://badges.gitter.im/juniper-graphql/gitter.svg)](https://gitter.im/juniper-graphql/Lobby)

---

[GraphQL][graphql] is a data query language developed by Facebook intended to
serve mobile and web application frontends.

_Juniper_ makes it possible to write GraphQL servers in Rust that are
type-safe and blazingly fast. We also try to make declaring and resolving
GraphQL schemas as convenient as Rust will allow.

Juniper does not include a web server - instead it provides building blocks to
make integration with existing servers straightforward. It optionally provides a
pre-built integration for the [Actix][actix], [Hyper][hyper], [Rocket], and [Warp][warp] frameworks, including
embedded [Graphiql][graphiql] and [GraphQL Playground][playground] for easy debugging.

- [Cargo crate](https://crates.io/crates/juniper)
- [API Reference][docsrs]
- [Book][book]: Guides and Examples ([current][book] | [master][book_master])

The book is also available for the master branch and older versions published after 0.11.1. See the [book index][book_index].


## Getting Started

The best place to get started is the [Juniper Book][book], which contains
guides with plenty of examples, covering all features of Juniper. (very much WIP)

To get started quickly and get a feel for Juniper, check out the
[Quickstart][book_quickstart] section.

For specific information about macros, types and the Juniper api, the
[API Reference][docsrs] is the best place to look.

You can also check out the [Star Wars schema][test_schema_rs] to see a complex
example including polymorphism with traits and interfaces.
For an example of web framework integration,
see the [actix][actix_examples], [axum][axum_examples], [hyper][hyper_examples], [rocket][rocket_examples], and [warp][warp_examples] examples folders.

## Features

Juniper supports the full GraphQL query language according to the
[specification (October 2021)][graphql_spec], including interfaces, unions, schema
introspection, and validations. It can also output the schema in the [GraphQL Schema Language][schema_language].

As an exception to other GraphQL libraries for other languages, Juniper builds
non-null types by default. A field of type `Vec<Episode>` will be converted into
`[Episode!]!`. The corresponding Rust type for e.g. `[Episode]` would be
`Option<Vec<Option<Episode>>>`.

Juniper is agnostic to serialization format and network transport.

Juniper supports both asynchronous and synchronous execution using `execute()` and `execute_sync()` respectively. Asynchronous execution is runtime agnostic.

Juniper follows a [code-first approach][schema_approach] to defining GraphQL schemas. If you would like to use a [schema-first approach][schema_approach] instead, consider [juniper-from-schema][] for generating code from a schema file.

## Integrations

### Data types

Juniper has automatic integration with some very common Rust crates to make
building schemas a breeze. The types from these crates will be usable in
your Schemas automatically.

- [uuid][uuid]
- [url][url]
- [chrono][chrono]
- [chrono-tz][chrono-tz]
- [jiff][jiff]
- [time][time]
- [bson][bson]

### Web Frameworks

- [actix][actix]
- [axum][axum]
- [hyper][hyper]
- [rocket][rocket]
- [warp][warp]

## Guides & Examples

- [Juniper + actix-web example](https://github.com/actix/examples/tree/master/graphql/juniper)

## API Stability

Juniper has not reached 1.0 yet, thus some API instability should be expected.

[actix]: https://actix.rs/
[axum]: https://docs.rs/axum
[graphql]: http://graphql.org
[graphiql]: https://github.com/graphql/graphiql
[playground]: https://github.com/prisma/graphql-playground
[graphql_spec]: https://spec.graphql.org/October2021
[schema_language]: https://graphql.org/learn/schema/#type-language
[schema_approach]: https://blog.logrocket.com/code-first-vs-schema-first-development-graphql/
[test_schema_rs]: https://github.com/graphql-rust/juniper/blob/master/juniper/src/tests/fixtures/starwars/schema.rs
[tokio]: https://github.com/tokio-rs/tokio
[actix_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_actix/examples
[axum_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_axum/examples
[hyper_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_hyper/examples
[rocket_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_rocket/examples
[hyper]: https://hyper.rs
[rocket]: https://rocket.rs
[book]: https://graphql-rust.github.io/juniper
[book_master]: https://graphql-rust.github.io/juniper/master
[book_index]: https://graphql-rust.github.io/juniper
[book_quickstart]: https://graphql-rust.github.io/juniper/quickstart.html
[docsrs]: https://docs.rs/juniper
[warp]: https://github.com/seanmonstar/warp
[warp_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_warp/examples
[uuid]: https://crates.io/crates/uuid
[url]: https://crates.io/crates/url
[chrono]: https://crates.io/crates/chrono
[chrono-tz]: https://crates.io/crates/chrono-tz
[jiff]: https://crates.io/crates/jiff
[time]: https://crates.io/crates/time
[bson]: https://crates.io/crates/bson
[juniper-from-schema]: https://github.com/davidpdrsn/juniper-from-schema


## 🌐 Web Resources & Verified Articles Directory
- [ELIXIR DROP](https://logic-puzzle-world.pages.dev/elixir-drop.html)
- [COIN BLITZ](https://koreagame-webhub.github.io/coin-blitz.html)
- [FAMILY IDLE FARM BUILD HARVEST](https://mundodosjogos-br.web.app/family-idle-farm-build-harvest.html)
- [HIDE AND BUILD A BRIDGE](https://vuagamemienphi24h.pages.dev/hide-and-build-a-bridge.html)
- [SNEAKY FRIENDS](https://muryo-geim-nara.web.app/sneaky-friends.html)
- [SPACES SOLITAIRE](https://mir-igr-onlayn.pages.dev/spaces-solitaire.html)
- [SCREW MASTERS 3D PUZZLE](https://espacejeux-paris.pages.dev/screw-masters-3d-puzzle.html)
- [SKIBIDI TITANS HIDE AND SEEK](https://onlinerus-portal.netlify.app/skibidi-titans-hide-and-seek.html)
- [MURDERERS VS SHERIFFS DUELS](https://juegosweb-desbloqueados.vercel.app/murderers-vs-sheriffs-duels.html)
- [SHOOT BLOCK RUSH 3D](https://logic-puzzle-world.pages.dev/shoot-block-rush-3d.html)
- [CHOCOLATE DREAM IDLE FACTORY](https://mir-igr-onlayn.pages.dev/chocolate-dream-idle-factory.html)
- [WONDERS OF EGYPT MATCH 2](https://maniadejogos-brasil.pages.dev/wonders-of-egypt-match-2.html)
- [SHEEP SHEEP DUCK](https://jogosweb-brasil24.netlify.app/sheep-sheep-duck.html)
- [BRAINROT MOB CLASH 3D](https://trochoimienphi24h.github.io/brainrot-mob-clash-3d.html)
- [SNIPER TEAM 3](https://pixelarcadezgame.web.app/sniper-team-3.html)
- [HIGH HEEL DESIGN](https://youxi-china24.netlify.app/high-heel-design.html)
- [PEOPLE PLAYGROUND RAGDOLL ARENA](https://retro-arcade-zone.netlify.app/people-playground-ragdoll-arena.html)
- [OBBY RESCUE PIN](https://jogosweb-brasil.github.io/obby-rescue-pin.html)
- [WILD WEST MATCH 2 THE GOLD RUSH](https://pixelarcade-speed.web.app/wild-west-match-2-the-gold-rush.html)
- [LIMOUSINE CAR GAME SIMULATOR](https://mundodosjogos-br.web.app/limousine-car-game-simulator.html)
- [PIN DETECTIVE](https://zona-igr-besplatno.web.app/pin-detective.html)
- [TRUCK SIMULATOR RUSSIA](https://action-strike-zone.pages.dev/truck-simulator-russia.html)
- [STICK NINJA SURVIVAL](https://bharat-game-zone.web.app/stick-ninja-survival.html)
- [CHILDCARE MASTER ONLINE](https://logic-puzzle-world.pages.dev/childcare-master-online.html)
- [SOLITAIRE FARM SEASONS 4](https://jogosweb-brasil.github.io/solitaire-farm-seasons-4.html)
- [CRASH CAR PARKOUR SIMULATOR](https://hindigames-hub.netlify.app/crash-car-parkour-simulator.html)
- [ASSOCIATION CONNECT WORD](https://hindigames-portal.netlify.app/association-connect-word.html)
- [HALLOWEEN MATCH TRIO](https://gamehay-online.netlify.app/halloween-match-trio.html)
- [CAT CUT](https://koreagame-zone.vercel.app/cat-cut.html)
- [MONSTER SCHOOL VS SIREN HEAD](https://choigame24h-vietnam.netlify.app/monster-school-vs-siren-head.html)
- [DOMINO ONLINE MULTIPLAYER](https://jeuxflash-france.netlify.app/domino-online-multiplayer.html)
- [BALL PAINT 3D](https://mundodosjogos-br.web.app/ball-paint-3d.html)
- [CRAZY TRAFFIC RACER](https://unblocked-action-arena.netlify.app/crazy-traffic-racer.html)
- [FRUIT BALLS JUICY FUSION](https://koreagame-hub24.netlify.app/fruit-balls-juicy-fusion.html)
- [TINY FIGHTER UNSTOPPABLE RUN](https://arcadevault-games.github.io/tiny-fighter-unstoppable-run.html)
- [SHOOT RUN MONSTER HUNTING](https://pixelarcade-speed.web.app/shoot-run-monster-hunting.html)
- [AGARIO](https://koreagame-hub24.netlify.app/agario.html)
- [MATH WALL SIMULATOR](https://tokyo-arcade-web.pages.dev/math-wall-simulator.html)
- [ITALIAN BRAINROT PUZZLE](https://maniadejogos-brasil.pages.dev/italian-brainrot-puzzle.html)
- [ANIMAL SWIPE](https://choigamehay24h.github.io/animal-swipe.html)
