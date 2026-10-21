# Node.js 与 npm 产物

P7 已实现本地 npm 产物准备，尚未发布。包名可用性和发布权限仍待核验。实际执行证据见[验收记录](/zh/development/formatter-acceptance#p7-node-acceptance)。

## 文本与配置 API

`import { format } from 'gdfmt'` 和 `const { format } = require('gdfmt')` 提供相同的四个函数：

| API                                 | 契约                                               |
| ----------------------------------- | -------------------------------------------------- |
| `formatSync(source, options?)`      | 在调用线程执行经过验证的格式化                     |
| `format(source, options?)`          | Promise 返回相同结果，核心计算在 libuv worker 执行 |
| `resolveConfigSync(path, options?)` | 显式解析文件系统配置                               |
| `resolveConfig(path, options?)`     | 在 libuv worker 中解析配置                         |

文本选项为 `{ language: { godotVersion }, format: { ...layoutOptions }, sourceId }`。默认值、语言版本和布局范围与 Rust 及[配置 Schema](/zh/guide/configuration)一致。拒绝未知字段、显式 null、非 JSON 值和未配对 Unicode surrogate。非法选项抛出异常或拒绝 Promise；源码错误和验证失败返回 `{ schemaVersion: 1, spanEncoding: 'utf8Bytes', output: null, changed: false, diagnostics }`，成功时包含已验证的 `output` 和 `changed`。

诊断 Span 为半开 UTF-8 字节偏移，包含 BOM 和物理 CRLF 字节，不等于 JavaScript 字符串下标。使用 `Buffer.from(source).subarray(span.start, span.end)` 读取对应原始字节；编辑器集成应显式转换为 UTF-16。

文本 API 不发现配置，也不写文件。配置解析选项为 `{ cwd, config }`：`cwd` 必须绝对，默认在调用时捕获当前目录；`config` 为可选的显式本地 JSON 路径。结果包含 `config`、应用顺序 `files`、`editorconfigFiles`、字段 `sources` 和 `ignored`。向文本 API 只传入 `resolution.config.language` 和 `resolution.config.format`，并自行处理 `ignored`。

npm 的 `gdfmt` 命令直接调用与独立 CLI 相同的 Rust runner，保留 stdout/stderr、stdin、reporter、发现、忽略、受保护写入和退出码优先级。`gdfmt/schema.json` 导出生成的配置 Schema。

## 拟定平台矩阵

Node 范围为 `^22.13.0 || ^24.0.0`，使用 Node-API 4。CI 在每个平台固定运行 **22.13.0** 和 **24.0.0**。其他 Node 主版本和未列架构不属于声明范围。

| 平台包                  | Rust target                 |
| ----------------------- | --------------------------- |
| `@gdfmt/darwin-arm64`   | `aarch64-apple-darwin`      |
| `@gdfmt/darwin-x64`     | `x86_64-apple-darwin`       |
| `@gdfmt/win32-x64-msvc` | `x86_64-pc-windows-msvc`    |
| `@gdfmt/linux-x64-gnu`  | `x86_64-unknown-linux-gnu`  |
| `@gdfmt/linux-x64-musl` | `x86_64-unknown-linux-musl` |

根包以相同精确版本固定五个平台 optional dependency。加载器选择 OS/CPU/libc，并校验平台 manifest、编译版本与 target；缺失、版本错误、不支持或损坏的 native 包明确失败。安装不执行生命周期脚本，不回退编译，也不额外下载二进制；应启用 optional dependencies。Linux glibc 产物构建于 Ubuntu 24.04，更旧 glibc 尚未验证；macOS/Windows 兼容范围暂限 CI runner，尚无更广泛系统版本测试。

## 本地产物门禁与恢复

仓库脚本均为 TypeScript，通过 `tsx` 执行：

```sh
pnpm install --frozen-lockfile
pnpm typecheck
pnpm test:npm
pnpm verify:npm --local
```

`pnpm test:npm [rust-target]` 构建 release 绑定和独立 CLI，打包根包及平台包，再通过隔离本地 registry 和空缓存安装根 tarball。消费者 PATH 只包含 Node。Vitest 的 TypeScript 测试对照 262 个脚本的五个语言 profile，另检查 Unicode/保护字节、选项、配置、CLI、worker 调度和加载失败。`.mts` / `.cts` 消费测试通过已安装的 export map 验证 ESM/CJS 声明。构建需要 Rust，消费 tarball 不需要。

`.github/workflows/npm.yml` 覆盖十组平台/运行时组合，包括 Alpine musl，并上传 tarball 与回执。`pnpm verify:npm` 要求十组回执齐全、根 tarball 字节完全一致、平台版本精确相同，并生成 SHA-512。`--local` 仅检查当前平台/运行时，明确记录为未完成完整矩阵。包和报告位于忽略的 `target/npm/`，CI 矩阵下载至 `target/npm-ci/`。

当前没有发布 job。未来发布前应核验名称/权限、完成全矩阵并冻结这些确切 tarball 和哈希。先发布平台包、核验 registry 版本和 integrity，再发布根包，最后调整 tag。中断后先检查 registry，仅以相同版本和字节补传缺失包，不重建或覆盖已有版本；若已有 integrity 不符，停止该次发布并使用新版本。若根包已发布但选定平台缺失，加载器会明确失败；应补齐缺失的不可变平台包，或将根包 tag 恢复至上次完整版本。部分矩阵不得推广。实际发布与 registry 恢复尚未执行，需另行安排发布任务。
