# PT Analytics Next

**利润收割者竞速分析 · Rust × Tauri 2**

PT Analytics Next 是独立的 Warframe 利润收割者日志分析工具。使用原项目 GPL Rust 核心的事件识别及四阶段计时逻辑，重构日志监听、异常恢复与本地存储，使用 Tauri 2 和独立的 HTML/CSS/JavaScript 前端。

[下载 Windows 版本](https://github.com/nekox41/PT-Analytics-Next/releases) · [构建状态](https://github.com/nekox41/PT-Analytics-Next/actions)

## 功能

- 自动读取 `%LOCALAPPDATA%\Warframe\EE.log`；也可指定日志路径。
- 无论先启动游戏还是软件，均回放完整历史日志，再持续监听增量。
- 字节级完整行缓冲、UTF-8 异常隔离、文件身份及读取锚点检测、自动重连。
- 六项汇总指标、四阶段详情、护盾属性和破腿分段；异常或中止记录有标记。
- 本地 SQLite 存储、重复回放去重、收藏、重命名、删除、搜索和排序。
- 平均耗时、PB、任务耗时趋势图；深色和浅色主题。
- 日志监听状态、同步进度、异常行计数及读取错误。
- 不需要原版 Flutter 应用、在线账号或远程服务。

## 使用

1. 下载 Windows x64 安装包或便携 EXE（需要 Microsoft Edge WebView2 Runtime；安装包支持其引导安装）。
2. 运行软件，启动 Warframe 并完成利润收割者任务；已有日志中的任务也会自动导入。
3. 如未找到日志，在设置中指定 EE.log 的绝对路径。
4. 数据保存在应用本地数据目录下的 runs.sqlite3 中。删除记录会保留去重标记，避免重启时重新导入。

## 开发

Windows 需要 Node.js 22、Rust stable、Visual Studio C++ Build Tools、WebView2 Runtime。

```sh
npm install
npm run icons
npm run check
cargo test --manifest-path src-tauri/Cargo.toml --release
npm run dev
npm run build -- --bundles nsis
```

## 项目结构

```
src-tauri/src/
  models/       # GPL Rust 数据模型
  parser/       # GPL 战斗事件与阶段解析
  engine.rs     # 解析状态机与异常隔离
  tail.rs       # 文件跟踪与完整行缓冲
  store.rs      # SQLite 持久化与去重
  main.rs       # Tauri 命令与监听线程
ui/             # 独立前端
.github/workflows/release.yml
```

目前以 Windows x64 为首发平台。首版暂未实现原应用的在线排行榜/提交、账号同步、截图及全局快捷键。计时规则仍基于上游日志标记；真实游戏日志回归和跨平台打包需要进一步验证。数据库使用新的结构，不会覆盖原版应用数据。

## 许可证

GPL-3.0-or-later。保留 [Profit-Taker-Analytics Rust 核心](https://github.com/nekox41/Profit-Taker-Analytics/tree/79001b346137bf5d87111015f418c89f634ec6b4/rust_core) 的作者与授权声明。感谢 Basi (Enrique Rodrigues)、Kalaay、scamcat、RemasteredArch、Mebius、Reava、Neppkun 及其他贡献者。详见 [NOTICE.md](NOTICE.md) 与 [LICENSE](LICENSE)。

本应用独立实现前端，未分发原项目的专有应用组件。非 Warframe 官方产品。
