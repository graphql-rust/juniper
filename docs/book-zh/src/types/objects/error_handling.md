# 错误处理

> [types/objects/error_handling.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/types/objects/error_handling.md)
> <br />
> commit 29025e6cae4a249fa56017dcf16b95ee4e89363e

Rust 将错误组合成[两个主要类别](https://rustbook.budshome.com/ch09-00-error-handling.html)： `Result<T, E>` 处理`可恢复错误`，`panic!` 处理`不可恢复错误`。Juniper 对`不可恢复错误`不做处理；`不可恢复错误`将上溯到集成 Juniper的框架，然后错误在框架层次有希望得到处理。

对于`可恢复错误`，Juniper 能够完善地地处理内建的 `Result` 类型，你可以使用 `?` 操作符或者 `try!` 宏（macro）来让程序按照预期设定工作：

```rust
# extern crate juniper;
use std::{
    str,
    path::PathBuf,
    fs::{File},
    io::{Read},
};
use juniper::FieldResult;

struct Example {
    filename: PathBuf,
}

#[juniper::object]
impl Example {
    fn contents() -> FieldResult<String> {
        let mut file = File::open(&self.filename)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }

    fn foo() -> FieldResult<Option<String>> {
      // 随意的无效字节值
      let invalid = vec![128, 223];

      match str::from_utf8(&invalid) {
        Ok(s) => Ok(Some(s.to_string())),
        Err(e) => Err(e)?,
      }
    }
}

# fn main() {}
```

`FieldResult<T>` 是 `Result<T, FieldError>` 的别名，其是所有字段都必须返回的错误类型。通过使用 `?` 操作符或者 `try!` 宏（macro），任何实现了 `Display` 特性的类型——这些既是当前绝大多数错误类型——这些错误会自动转换为 `FieldError`。

当字段返回错误时，字段的结果将被 `null` 替换，然后在响应的顶层附加一个名为 `errors` 的对象，最后继续执行程序。例如，基于前述的示例代码，在 GraphQL 做如下查询：

```graphql
{
  example {
    contents
    foo
  }
}
```

若果 `str::from_utf8` 导致了 `std::str::Utf8Error` 错误, 将返回以下内容：

!文件名 错误的可空字段的响应

```js
{
  "data": {
    "example": {
      contents: "<Contents of the file>",
      foo: null,
    }
  },
  "errors": [
    "message": "invalid utf-8 sequence of 2 bytes from index 0",
    "locations": [{ "line": 2, "column": 4 }])
  ]
}
```

如果非空字段返回错误，如同上述示例代码，`null` 值将传播到第一个可空的父字段；如果没有可空字段，则传播到根（root）内名为 `data` 的对象。

举例，执行如下查询：

```graphql
{
  example {
    contents
  }
}
```

若果上述代码中的 `File::open()` 导致 `std::io::ErrorKind::PermissionDenied` 错误，将返回以下内容：

!文件名 没有可空父子段的非空字段的响应

```js
{
  "errors": [
    "message": "Permission denied (os error 13)",
    "locations": [{ "line": 2, "column": 4 }])
  ]
}
```

## 结构化错误

有些情况下，有必要向客户端返回附加的结构化错误信息。可以通过实现 [`IntoFieldError`](https://docs.rs/juniper/latest/juniper/trait.IntoFieldError.html) 来解决：

```rust
# #[macro_use] extern crate juniper;
enum CustomError {
    WhateverNotSet,
}

impl juniper::IntoFieldError for CustomError {
    fn into_field_error(self) -> juniper::FieldError {
        match self {
            CustomError::WhateverNotSet => juniper::FieldError::new(
                "不存在任何东东",
                graphql_value!({
                    "type": "NO_WHATEVER"
                }),
            ),
        }
    }
}

struct Example {
    whatever: Option<bool>,
}

#[juniper::object]
impl Example {
    fn whatever() -> Result<bool, CustomError> {
      if let Some(value) = self.whatever {
        return Ok(value);
      }
      Err(CustomError::WhateverNotSet)
    }
}

# fn main() {}
```

指定的结构化错误信息被包含在名为 [`extensions`](https://facebook.github.io/graphql/June2018/#sec-Errors) 的键值中：

```js
{
  "errors": [
    "message": "不存在任何东东",
    "locations": [{ "line": 2, "column": 4 }]),
    "extensions": {
      "type": "NO_WHATEVER"
    }
  ]
}
```
