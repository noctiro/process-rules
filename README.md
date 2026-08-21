# [Mihomo](https://github.com/MetaCubeX/mihomo) 跨平台进程规则

按软件适用地区整理的 `PROCESS-NAME` 规则。生成结果发布在 `generated` 分支，可以直接通过 URL 引用。

## 使用

在 [`generated/mihomo`](https://github.com/noctiro/process-rules/tree/generated/mihomo) 中选择平台和规则集合。可以使用 GitHub Raw 或 jsDelivr：

```text
https://raw.githubusercontent.com/noctiro/process-rules/generated/mihomo/<platform>/<collection>.list
https://cdn.jsdelivr.net/gh/noctiro/process-rules@generated/mihomo/<platform>/<collection>.list
```

Mihomo 配置示例：

```yaml
rule-providers:
  process-contain-cn-mainland:
    type: http
    behavior: classical
    format: text
    url: https://cdn.jsdelivr.net/gh/noctiro/process-rules@generated/mihomo/windows/contain-cn-mainland.list
    path: ./ruleset/process-contain-cn-mainland.list
    interval: 86400

rules:
  - RULE-SET,process-contain-cn-mainland,DIRECT
```

将 `DIRECT` 替换为需要的策略或策略组。

### 角色

| 平台 | 内容 |
| --- | --- |
| `windows` | Windows 可执行文件名 |
| `android` | Android 应用包名 |
| `linux` | Linux 进程名 |
| `macos` | macOS 可执行文件名 |
| `all` | 所有平台去重合并 |

### 规则集合

| 集合 | 内容 |
| --- | --- |
| `only-<region>` | 只面向该角色的应用 |
| `contain-<region>` | 包含该角色的应用 |
| `not-contain-<region>` | 有明确角色但不包含该角色的应用 |
| `any` | 不限制角色的应用 |

`any` 不表示全部应用。`only-<region>` 是 `contain-<region>` 的子集；两者使用不同策略时，应先放 `only-<region>`。

当前文件及其 SHA-256 校验值见 [`manifest.json`（GitHub Raw）](https://raw.githubusercontent.com/noctiro/process-rules/generated/manifest.json)或 [`manifest.json`（jsDelivr）](https://cdn.jsdelivr.net/gh/noctiro/process-rules@generated/manifest.json)。固定版本见 [Releases](https://github.com/noctiro/process-rules/releases)。

规则只使用能够唯一归属到具体软件的进程名和 Android 包名。提交规则见 [贡献者指南](CONTRIBUTING.md)。代码、规则数据和生成文件采用 [GNU GPLv3 或更高版本](LICENSE)。
