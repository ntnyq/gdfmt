# 快速开始

gdfmt 是基于 Rust 的 GDScript 格式化器。目前已实现词法分析、无损脚本与表达式解析、配置 Schema 和基础 CLI。P4 脚本格式化与验证后的 CLI 已可用；源码构建 alpha 已按明确排除项验收；formatter-off/on 保护区域已可用。Node/npm API 与本地打包已实现，完整平台验收待 CI；lint 和语义修复不在范围内，crates 暂未发布。

::: warning 开发状态
默认命令格式化脚本，包含注释、lambda、声明和 suite；`check` 只读，`check --syntax-only` 运行 parser。固定项目试用及风格差异已记入 [P4 验收](/zh/development/formatter-acceptance)，使用前请阅读 [P4 格式化范围](/zh/reference/formatting)。`lex` 通过不代表语法正确。
:::

## 从源码构建

克隆仓库并使用 `rust-toolchain.toml` 固定的开发工具链。workspace 使用 edition 2024，最低支持 Rust 1.88。

```sh
git clone https://github.com/ntnyq/gdfmt.git
cd gdfmt
cargo build --workspace --locked
cargo run -p gdfmt_cli -- --help
```

## 查看 token

```sh
cargo run -p gdfmt_cli -- lex tests/compatibility/fixtures/containers.gd
cargo run -p gdfmt_cli -- lex tests/compatibility/fixtures --reporter json
cargo run -p gdfmt_cli -- lex - --reporter json < script.gd
```

`lex` 只读文件，不修改源码。支持 `.gd` 文件、目录、多路径，以及单独使用的 stdin。显式文件与目录遍历遵守本地 Git/ignore 文件和 JSON ignore，详见[输入选择](/zh/guide/configuration#忽略规则与输入选择)。内置排除包括 `.git`、`.godot`、`node_modules` 和 `target`。

| 退出码 | 含义                  |
| ------ | --------------------- |
| `0`    | 未发现词法错误        |
| `1`    | 发现词法错误          |
| `2`    | 参数、配置或 I/O 错误 |

JSON reporter 使用 `schemaVersion: 1` 和 `spanEncoding: "utf8Bytes"`。span 是 UTF-8 字节半开区间 `[start, end)`，虚拟 token 不占字节。

## 配置项目

```sh
cargo run -p gdfmt_cli -- init
cargo run -p gdfmt_cli -- schema
cargo run -p gdfmt_cli -- explain-config .
```

`init` 创建 `.gdfmtrc.json`，拒绝覆盖已有文件。[配置说明](/zh/guide/configuration)介绍发现规则、默认值、严格校验与字段来源。宽度、缩进、整文件换行和空行配置均已生效，详见[格式化](/zh/reference/formatting)。

## 在 Rust 中使用解析器

完整脚本使用 [`parse`](/zh/reference/parsing)，单表达式使用 [`parse_expression`](/zh/reference/expressions)。两者都返回无损 CST 和结构化诊断。[语言版本 profiles](/zh/reference/language-versions)覆盖 Godot 4.3–4.7 的已核验差异，不代表完整兼容性。

## 开发文档站点

使用 Node.js 22 或更高版本与 pnpm：

```sh
cd docs
pnpm install --frozen-lockfile
pnpm dev
pnpm build
pnpm preview
```

英文页面位于站点根目录；中文页面在 `zh/` 下保持相同路径。语言菜单会切换到当前主题的对应语言。共享资源放在 `public/`，站点配置与主题扩展放在 `.vitepress/`。

如果部署在子路径，请为构建和预览传入相同 base，例如 `pnpm build --base /gdfmt/` 和 `pnpm preview --base /gdfmt/`。

贡献前可阅读[架构](/zh/development/)、[兼容性](/zh/development/compatibility)与[实施计划](/zh/development/implementation-plan)。
