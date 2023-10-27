# Multiple operations per request

The GraphQL standard generally assumes there will be one server request for each client operation you want to perform (such as a query or mutation). This is conceptually simple but has the potential to be inefficient.

Some client libraries such as [apollo-link-batch-http](https://www.apollographql.com/docs/link/links/batch-http.html) have added the ability to batch operations in a single HTTP request to save network round-trips and potentially increase performance. There are some [tradeoffs](https://blog.apollographql.com/batching-client-graphql-queries-a685f5bcd41b) that should be considered before batching requests.

Juniper's server integration crates support multiple operations in a single HTTP request using JSON arrays. This makes them compatible with client libraries that support batch operations without any special configuration.

Server integration crates maintained by others are **not required** to support batch requests. Batch requests aren't part of the official GraphQL specification.

Assuming an integration supports batch requests, for the following GraphQL query:

```graphql
{
  hero {
    name
  }
}
```

The json data to POST to the server for an individual request would be:

```json
{
  "query": "{hero{name}}"
}
```

And the response would be of the form:

```json
{
  "data": {
    "hero": {
      "name": "R2-D2"
    }
  }
}
```

If you wanted to run the same query twice in a single HTTP request, the batched json data to POST to the server would be:

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

And the response would be of the form:

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
