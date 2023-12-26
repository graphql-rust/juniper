Error handling
==============

Error handling in [GraphQL] can be done in multiple ways. We will cover the two different error handling models mostly used: 
1. [Implicit field results](field.md). 
2. [Explicit errors backend by GraphQL schema](schema.md). 

Choosing the right error handling method depends on the requirements of the application and the concrete error happening. Investigating both approaches is beneficial.




## Comparison

The [first approach](field.md) (where every error is a [field error][1]) is easier to implement. However, clients won't know what errors may occur and instead will have to infer what happens from the [error message][2]. This is brittle and could change over time due to either clients or server changing. Therefore, extensive integration testing between clients and server is required to maintain the implicit contract between the two.

[Encoding non-critical errors in a GraphQL schema](schema.md) makes the contract between clients and the server explicit. This allows clients to understand and handle these errors correctly and the server to know when changes are potentially breaking clients. However, encoding this error information into a [GraphQL schema][8] requires additional code and up-front definition of non-critical errors.




[GraphQL]: https://graphql.org

[1]: https://spec.graphql.org/October2021#sec-Errors.Field-errors
[2]: https://spec.graphql.org/October2021/#sel-GAPHRPDCAACCyD57Z
[8]: https://graphql.org/learn/schema
