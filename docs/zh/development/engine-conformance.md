# 固定引擎一致性测试

`cargo xtask conformance` 使用 `tests/compatibility/engine.json` 的固定版本矩阵，对完整脚本执行引擎检查和对应 Rust grammar profile 检查。保留 4.7.2 基线，增加 4.3–4.6 的首个 stable 发布版，以验证特性引入时的边界。

| Profile | 固定 tag                                                                       | 精确 `--version`                  |
| ------- | ------------------------------------------------------------------------------ | --------------------------------- |
| 4.3     | [4.3-stable](https://github.com/godotengine/godot/releases/tag/4.3-stable)     | `4.3.stable.official.77dcf97d8`   |
| 4.4     | [4.4-stable](https://github.com/godotengine/godot/releases/tag/4.4-stable)     | `4.4.stable.official.4c311cbee`   |
| 4.5     | [4.5-stable](https://github.com/godotengine/godot/releases/tag/4.5-stable)     | `4.5.stable.official.876b29033`   |
| 4.6     | [4.6-stable](https://github.com/godotengine/godot/releases/tag/4.6-stable)     | `4.6.stable.official.89cea1439`   |
| 4.7     | [4.7.2-stable](https://github.com/godotengine/godot/releases/tag/4.7.2-stable) | `4.7.2.stable.official.ed1daf0bf` |

每个 pin 记录完整 commit、tag、精确版本字符串和官方 `SHA512-SUMS.txt` 中的 Linux archive 摘要。默认 profile 的 pin 还必须与主 manifest 的基线相符。CI 的五个独立作业从同一 manifest 读取 pin，下载、校验 SHA-512、执行引擎检查并分别上传报告；不另维护一套 workflow 版本与摘要。

```sh
cargo xtask compatibility
cargo xtask conformance --godot /path/to/pinned/godot
# 从精确引擎版本选择对应 profile，也可显式约束 profile
cargo xtask conformance --language 4.3 --godot /path/to/godot-4.3 --report target/conformance/report-4.3.json
GODOT_BIN=/path/to/godot-4.6 cargo xtask conformance --language 4.6
```

每次命令检查一个引擎。`--language` 与实际二进制不符、非固定 patch、未知版本和自定义构建均失败。需要对应 editor 二进制；harness 不会自动下载或选择 latest。常规 Rust 测试不需要安装 Godot。

## 执行与判定

每个用例建立独立临时工程，原样写入 `case.gd`，执行 `--headless --path <project> --check-only --script res://case.gd`。fixture 不作为运行时脚本执行；工程与日志随后清理。各 fixture 不共享 class registry、导入结果或资源文件。当前均为单文件脚本，资源依赖和多文件工程尚未覆盖。

引擎 check-only 会进入语义分析，部分版本遇到 Parse Error 仍返回 0。harness 同时检查脚本错误、目标 `res://case.gd`、加载失败日志、正常退出状态和每个负例固定的诊断片段。4.3/4.4 的输出即使设置 `NO_COLOR` 仍可能带 ANSI 颜色码；判定时移除 CSI 控制序列，报告保留原始 stdout/stderr。每次运行先检查有效与语法错误控制脚本；错误版本、忽略参数的空跑、崩溃、启动错误都会失败。进程有 15 秒超时和 1 MiB 标准输出/错误限制；输出写入临时文件避免管道阻塞。

`engine.json` schema 2 覆盖主 manifest 的全部 **262 个 script fixtures × 5 个版本，共 1310 次引擎检查**。每个 `expectations` 必须显式包含全部 profiles，分别登记分类和错误文本；没有继承或跳过规则。新增脚本未分类、重复分类、缺少 profile/pin、负例缺少诊断或 analyzer 缺少解释都会失败。单表达式与纯词法 fixtures 由 `compatibility` 检查；独立表达式不直接交给引擎。

| kind       | Rust parser | Godot | 说明                                                  |
| ---------- | ----------- | ----- | ----------------------------------------------------- |
| `accepted` | 接受        | 接受  | 可独立检查的有效脚本                                  |
| `syntax`   | 拒绝        | 拒绝  | 语法、版本 gate 或已有结构约束；要求 `engineError`    |
| `analyzer` | 接受        | 拒绝  | parser 范围外的语义错误；要求 `engineError` 和 `note` |

阶段由人工对照固定 parser/analyzer 源码登记，不能根据引擎的 “Parse Error” 文案推断。4.3–4.5 各有 4 个 analyzer 用例；4.6/4.7 各有 5 个，新增 multiline hint 参数类型边界：低版本因不支持 hint 参数而语法拒绝，4.6 起由 analyzer 拒绝整数参数。这些边界单独检查，不计为语法一致性通过或跳过。

默认报告为 `target/conformance/report.json`。矩阵运行应通过 `--report` 指定不同文件，避免覆盖。schema 2 报告记录完整 commit/tag、引擎版本、profile、每个用例的分类、预期错误、parser 诊断、引擎退出码与原始日志及最终判定。用例不匹配仍输出报告并返回失败；版本、控制检查等基础设施失败直接返回失败。

## Gates 与解析修正

`gates` 为每个 `LanguageFeature` 指定有效与无效脚本。harness 强制有效脚本在引入版本前拒绝、从引入版本起接受；无效脚本在全部版本拒绝。缺少任一 feature 的正反例或分类与 `supports()` 不符都会失败。

| 引入版本 | 独立正反例                                                                                                              |
| -------- | ----------------------------------------------------------------------------------------------------------------------- |
| 4.4      | 类型化 Dictionary 的声明/参数/返回值/as/is 与错误类型参数数目；tool button 与缺少标题；warning start/restore 与缺少参数 |
| 4.5      | abstract 类/方法与错误函数体；普通/static/lambda 可变参数与类型化 rest/后续参数；file path 与错误 annotation 目标       |
| 4.6      | multiline hints 与错误 annotation 目标；无参数的旧写法在全部版本仍接受                                                  |

补充用例验证 warning restore 参数、重复 start、无 start 的 restore、跨函数范围和转义字符串，以及 file path 的可选参数。此次发现 parser 会误接受重复 start 和无对应 start 的 restore，现按源码顺序维护 warning 区域状态；名称不区分大小写，Unicode 转义、raw/triple 字符串的等价名称共享状态。start 可以持续到文件末尾，区域不会随函数或类作用域结束而清空。依据为固定 [4.4 parser](https://github.com/godotengine/godot/blob/4c311cbee68c0b66ff8ebb8b0defdd9979dd2a41/modules/gdscript/gdscript_parser.cpp) 的 `warning_ignore_region_annotations`。警告名称目录有效性和一般 annotation 值求解仍不属于此次结构检查。

此前修正的 `Array[int]([])` / `Dictionary[String, int]({})` 也在全部五个引擎上回归：值位置的 `[]` 是下标，不能直接调用该表达式；类型声明和 `as`/`is` 继续解析集合类型参数。依据为固定 [4.7.2 parser](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.cpp) 的 `parse_subscript` / `parse_call`。

## 声明、语句与恢复审计

在原有 79 个脚本上增加 156 个原创脚本，每个新增用例都对照五个固定二进制。按固定 [4.6 parser](https://github.com/godotengine/godot/blob/89cea143987d564363e15d207438530651d943ac/modules/gdscript/gdscript_parser.cpp) 的声明/语句入口逐项检查，并用 Rust 回归断言 CST 结构和作用域。此处是有边界的语法探测，不代表语言一致性的穷尽证明。

| 范围                     | 正反例证据（`fixtures/audit-*.gd`）                                                        |
| ------------------------ | ------------------------------------------------------------------------------------------ |
| 类头与继承               | 点分隔/路径基类、内部类头、重复类头、注释与类头顺序、非法集合基类                          |
| 类与成员                 | 单行内部类成员边界、匿名/命名 enum 冲突、上下文/保留名字、静态成员                         |
| 变量与常量               | 推断/显式类型、初始化、类体/局部上下文、缺名字/值、static 常量                             |
| enum 与 signal           | 值、尾逗号、重复 enum 条目、signal 类型参数、非法默认值/返回类型                           |
| 函数与参数               | 推断默认值、重复参数、无 body 声明、构造器、静态初始化                                     |
| 属性                     | body/委托 accessor、`get()`、static 属性、setter 返回值、混用、非法 setter 类型            |
| annotation 与注释        | 同行绑定、目标、跨 pass/注释字符串的待绑定 annotation、独立行、注释换行                    |
| suite 与条件语句         | 单行/嵌套复合语句、分号结束、块首/连续分号、孤立/重复分支、分支作用域                      |
| 循环与控制语句           | 类型化迭代器、作用域复用/冲突、break/continue 边界、lambda 重置循环上下文                  |
| return/assert/表达式语句 | 返回 lambda、assert 尾逗号/参数数量、breakpoint、独立表达式/lambda                         |
| 赋值                     | 全部复合赋值运算符、括号目标、调用作为目标、非法链式赋值                                   |
| match                    | 嵌套数组/字典、key-only/rest 模式、guard、绑定冲突、备选模式、循环上下文                   |
| lambda 作用域            | 之前/之后的局部声明、初始化时机、参数遮蔽、嵌套捕获、循环/guard/默认值/setter 上下文       |
| 恢复                     | 缺名字、操作数、参数、冒号/缩进，以及分支、模式、accessor、enum 条目、annotation、词法错误 |

修正包括：拒绝 `extends` 中的集合类型、块首分号、括号包裹的独立 lambda、类体注释字符串后的分号；接受 assert 尾逗号、setter 返回值和完整语句后的连续分隔符；恢复单行内部类边界、内部类头顺序、待绑定 annotation 的规则、lambda 的外层局部可见性；并将错误 accessor 保留在属性块内恢复。

五个 pin 均允许 `..` 后接 key-only 字典模式，例如 `{.., "key"}`，但拒绝后续带值的模式或重复 rest。该边界对应 `parse_match_pattern`，明确登记引擎行为，不用更严格的规则代替。

主 manifest 中有 19 个语法错误 fixture 指定 `recoveredDeclarations`。两条命令均要求这些名字在每个 profile 中恢复为 SourceFile 的直接声明；名字仅出现在错误节点、初始化表达式或内部类中不算通过。报告包含预期与缺失的恢复名字。未闭合字符串和分隔符仍检查拒绝及无损重建，但不保证后续声明恢复：多行字符串和分隔符续行可能覆盖看似独立的源码边界。

本轮在 macOS 运行全部五个固定引擎；Linux 引擎验证由 CI 矩阵执行。完整语言覆盖、通用 AST/字面量值比较、资源/继承分析、运行时等价和 formatter 一致性均未据此声明完成。

[P3 语法审计](/zh/development/syntax-audit)再增加 20 个原创回归，并按明确排除项验收 50 条语法契约；P4 再增加五个布局回归与两个项目试用回归，完整语料现为 262 个脚本。独立命令 `cargo xtask formatter-conformance` 使用同一固定引擎，以宽度 40 检查格式化后的有效脚本，报告包含格式化源码与日志，522 个格式化有效输出均通过，详见 [P4 验收记录](/zh/development/formatter-acceptance)。
