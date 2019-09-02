# 批量操作请求

> [advanced/multiple_ops_per_request.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/advanced/multiple_ops_per_request.md)
> <br />
> commit 9623e4d32694118e68ce8706f29e2cfbc6c5b6dc

GraphQL 标准通常假定，每个客户端操作（例如查询或变更）都有一次服务器请求。这在概念上很简单，但有可能效率低下。

一些客户端库——如[apollo-link-batch-http](https://www.apollographql.com/docs/link/links/batch-http.html)，已经在单个 HTTP 请求中添加了批量操作请求的功能，以便于节省网络往返请求并提高性能。当然，在批量操作请求之前，应该进行[权衡](https://blog.apollographql.com/batching-client-graphql-queries-a685f5bcd41b)。

Juniper 服务器集成包使用 JSON 数组支持单个 HTTP 请求中的批量操作请求，这样不需要任何特殊配置就能兼容支持客户端库的批量操作请求。

第三方维护的服务器集成包**不需要**支持批量操作请求，批量操作请求不属于 GraphQL 官方规范。

假定某个服务器集成支持批操作请求，现执行如下 GraphQL 查询：

```graphql
{
  hero {
    name
  }
}
```

单个请求 POST 到服务器的 json 数据是：

```json
{
  "query": "{hero{name}}"
}
```

单个请求响应数据如下：

```json
{
  "data": {
    "hero": {
      "name": "R2-D2"
    }
  }
}
```

如果你想在一个 HTTP 请求中运行两次相同的查询，那么要 POST 到服务器的批量 JSON 数据是：

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

批量操作请求响应数据如下：

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
