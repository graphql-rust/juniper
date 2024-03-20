Batching
========

The [GraphQL] standard generally assumes that there will be one server request per each client operation to perform (such as a query or mutation). This is conceptually simple but potentially inefficient.

Some client libraries (such as [`apollo-link-batch-http`][1]) have the ability to batch operations in a single [HTTP] request to save network round-trips and potentially increase performance. There are [some tradeoffs][3], though, that should be considered before [batching operations][2].

[Juniper]'s [server integration crates](index.md#officially-supported) support [batching multiple operations][2] in a single [HTTP] request out-of-the-box via [JSON] arrays. This makes them compatible with client libraries that support [batch operations][2] without any special configuration.

> **NOTE**: If you use a custom server integration, it's **not a hard requirement** to support [batching][2], as it's not a part of the [official GraphQL specification][0].

Assuming an integration supports [operations batching][2], for the following GraphQL query:
```graphql
{
  hero {
    name
  }
}
```

The [JSON] `data` to [POST] for an individual request would be:
```json
{
  "query": "{hero{name}}"
}
```
And the response would be in the form:
```json
{
  "data": {
    "hero": {
      "name": "R2-D2"
    }
  }
}
```

However, if we want to run the same query twice in a single [HTTP] request, the batched [JSON] `data` to [POST] would be:
```json
[
  {
    "query": "{hero{name}}"
  },
  {
    "query": "{hero{name}}"
  }
]
```
And then, the response would be in the following array form:
```json
[
  {
    "data": {
      "hero": {
        "name": "R2-D2"
      }
    }
  },
  {
    "data": {
      "hero": {
        "name": "R2-D2"
      }
    }
  }
]
```




[GraphQL]: https://graphql.org
[HTTP]: https://en.wikipedia.org/wiki/HTTP
[JSON]: https://www.json.org
[Juniper]: https://docs.rs/juniper
[POST]: https://en.wikipedia.org/wiki/POST_(HTTP)

[0]: https://spec.graphql.org/October2021
[1]: https://www.apollographql.com/docs/link/links/batch-http.html
[2]: https://www.apollographql.com/blog/batching-client-graphql-queries
[3]: https://www.apollographql.com/blog/batching-client-graphql-queries#what-are-the-tradeoffs-with-batching
