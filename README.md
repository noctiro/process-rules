# process-rules generated rules

此分支由 GitHub Actions 从 `main` 分支自动生成，只保存当前可用的 Mihomo `PROCESS-NAME` rule provider。请勿直接修改此分支；规则源数据和生成器位于 [`main`](https://github.com/noctiro/process-rules)。

## 目录

```text
.
├── README.md
├── LICENSE
├── manifest.json
└── mihomo/
    ├── windows/
    ├── android/
    ├── linux/
    ├── macos/
    └── all/
```

每个平台目录包含 Mihomo classical `.list` rule provider。可用平台、集合、条目数和 SHA-256 以 [`manifest.json`](manifest.json) 为准。

使用方法、集合语义和配置示例见项目的[用户说明](https://github.com/noctiro/process-rules#readme)。
