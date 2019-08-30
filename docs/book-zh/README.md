# Juniper 中文手册（同步最新开发版）

Juniper 中文手册包含 `Juniper` 中文文档和代码示例，内容译自[官方文档](https://github.com/graphql-rust/juniper/tree/master/docs/book)。

💥 **更新时间：2019-08-30**

------

## 做贡献

### 需求

本手册由 [mdBook](https://github.com/rust-lang-nursery/mdBook) 编译而成。

如果已有 `Rust` 环境，安装 `mdBook` 请执行命令：

```bash
cargo install mdbook
```

### 启动本地测试服务器

启动持续编译手册并自动加载页面的本地测试服务器，执行命令：

```bash
mdbook serve
```

### 生成手册

将手册渲染输出为 `HTML`，执行命令：

```bash
mdbook build
```

输出目录为：`./docs`。

### 测试

测试手册中的所有代码示例，运行命令：

```bash
cd ./tests
cargo test
```

### 测试配置

手册中的所有 `Rust` 代码示例在 `CI` 上编译，使用了  [skeptic](https://github.com/budziq/rust-skeptic) 库。

## 声明

水平有限，错漏难免，欢迎指教；或请发 [issue到GitHub](https://github.com/zzy/juniper-book-zh)；或者直接联系。

> 电子邮箱：linshi@budshome.com；微信：cd-zzy；QQ：9809920。

感谢`graphql-rust/juniper 团队`的无私奉献。

💥 笔者无意侵犯任何人的权利和利益，故若有不适，请联系我。

------
