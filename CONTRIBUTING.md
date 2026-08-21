# 贡献者指南

提交规则前，请先确认推荐出口，以及至少一个平台的进程名或 Android 包名。未知的平台字段不要填写。PR 说明需附上可核查的依据，例如官方文档、安装包清单、软件包元数据、网络测试结果或系统进程信息。

## 收录范围

软件必须能由进程名或包名唯一识别，其网络流量也必须适合使用同一组出口规则。

不收录以下内容：

- 完全不需要联网，也没有任何联网功能的软件，包括纯离线游戏。
- 浏览器、WebView、终端、Shell、通用下载器，以及连接目标由用户决定的 SSH、RDP 等客户端。
- 允许用户自定义模型提供商、API 地址、工具或插件的 AI Agent。这类软件会通过同一进程连接不同目标，无法确定统一出口。
- `java`、`python`、`node`、`electron`、`svchost.exe` 等承载多个无关软件的运行时、宿主或系统进程。
- `app`、`main`、`helper`、`service`、`launcher`、`updater` 等无法指向特定软件的通用进程名。
- 核心流量需要不同出口，但共用同一进程，无法用一条进程规则处理的软件。
- 无法确认推荐出口的软件。

应用专用的辅助进程可以收录，但名称必须能看出所属应用，并与主进程使用相同的 `regions`。

## 出口地区

`regions` 表示流量应优先使用的代理出口。判断时先看登录、账号、内容、同步、匹配、游戏等核心业务由哪里的服务器提供。出口应尽量接近这些服务器，不是把所有可以连通的地区都列出来。

| 写法 | 含义 | 适用情况 |
| --- | --- | --- |
| `regions = ["cn-mainland", "hk"]` | 优先使用列出的出口 | 核心服务区域较少，可以完整列出 |
| `regions = { except = ["cn-mainland"] }` | 除列出地区外，其他出口都可优先使用 | 服务覆盖大部分地区，而且排除项可以完整确认 |
| `regions = "any"` | 没有稳定的出口差异 | 核心服务全球分布，或出口位置对服务没有明显影响 |

能连上服务，只能证明出口可用。打开应用、访问下载页或完成安装，也不能据此填写 `regions`。需要找到核心业务连接，再判断对应的服务器区域。

判断地区时遵循这些规则：

- 优先采用官方公布的服务器区域、服务区域和版本说明。没有官方说明时，可以结合核心域名、网络请求和多个出口的实际测试。
- IP 地理位置只适用于已经确认的核心业务服务器。不要根据 CDN、对象存储、更新、广告、遥测、崩溃报告或第三方 SDK 节点判断地区。
- 不同包名分别判断。名称里的 `global`、`international`、`cn`、`jp` 等字样只能作为线索。
- 同一包名如果允许选择多个互斥的服务器区域，填写这些区域的并集。如果软件会同时连接多个核心区域，而一条出口规则无法合理处理，则不收录。
- 商店上架国家、账号注册地区、语言、货币、域名后缀、用户所在地和公司注册地都不能单独作为依据。
- 区域范围或排除项没有查清时，不要扩大成 `any` 或 `{ except = [...] }`。先补充依据，无法确认则不收录。

## 规则格式

- `rules/` 只放直接子级 TOML 文件，不按平台、地区或策略建立子目录。
- 每个文件只在开头写一次 `version = 1`。
- 应用 ID 在整个仓库内唯一，只能使用小写 ASCII 字母、数字和单连字符。
- 一个应用只放在一个分类文件中，各平台写在同一张表内。至少填写一个平台，空数组直接省略。
- 地区使用 `cn-mainland` 或小写 ISO 3166-1 alpha-2 代码。数组不能为空，也不能有重复项。
- Windows、Linux 和 macOS 填写可执行文件 basename；Android 填写包名。
- 进程名不能包含路径、逗号、首尾空白、控制字符、glob 或正则。同一平台内不得有忽略大小写后相同的名称；跨平台同名必须使用相同的 `regions`。
- 不要提交窗口标题、个人信息或猜测值，也不要直接修改 `generated` 分支和 Release 资产。

```toml
version = 1

[example-app]
display_name = "Example App"
regions = ["cn-mainland"]
windows = ["ExampleApp.exe"]
android = ["com.example.app"]
linux = ["example-app"]
macos = ["ExampleApp"]
```

## 软件分类

分类只用于维护，不影响生成结果。每个应用按主要用途放入一个文件，不按公司、地区、平台、路由策略或附加功能分类。官方产品定位和用户打开软件后通常要完成的事情，是判断主要用途的依据。

| 文件 | 收录范围 |
| --- | --- |
| `ai.toml` | 生成式 AI、模型交互和本地模型管理 |
| `communication.toml` | 即时通信、邮件、会议、社交网络和社区服务 |
| `media.toml` | 音乐、视频、直播、阅读和媒体播放 |
| `creative.toml` | 图片、视频和音频内容创作与编辑 |
| `games.toml` | 游戏、游戏平台、启动器和云游戏 |
| `productivity.toml` | 办公、笔记、日历、任务、知识管理、文件同步和云盘 |
| `finance.toml` | 银行、支付、证券、数字资产和个人财务 |
| `commerce.toml` | 购物、外卖、本地生活和商家交易 |
| `business.toml` | 企业查询和商业信息服务 |
| `employment.toml` | 招聘、求职、实习和职业社交 |
| `real-estate.toml` | 房屋交易、租赁和房产信息服务 |
| `maps-travel.toml` | 地图、导航、出行、住宿和旅行预订 |
| `automotive.toml` | 汽车品牌服务、用车管理和汽车信息服务 |
| `education.toml` | 课程、教学、题库、语言学习、考试和学术工具 |
| `government.toml` | 政务、税务、社保和公共行政服务 |
| `network.toml` | 网络加速、代理、VPN、DNS、网络诊断和远程访问产品 |
| `security.toml` | 账号安全、身份验证、终端防护、反欺诈和隐私安全 |
| `health-fitness.toml` | 医疗健康、运动、健身和训练记录 |
| `logistics.toml` | 快递、寄件、包裹查询和物流服务 |
| `system.toml` | 输入法、应用商店、设备管理、厂商系统组件和系统工具 |
| `other.toml` | 符合收录条件，但尚无独立分类的软件 |

## 提交前检查

只修改规则数据时运行：

```shell
cargo run --locked -- validate
```

修改代码或 Workflow 时运行：

```shell
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo doc --locked --no-deps --all-features
cargo run --locked -- validate
```

`main` 更新后，Workflow 会重新生成并覆盖 `generated` 分支。`v*` Tag 会发布生成器和规则归档。这两条发布路径都要求规则库非空。所有贡献按 [GNU GPLv3 或更高版本](LICENSE) 发布。
