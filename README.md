# PT Analytics Next

**利润收割者竞速分析 · Rust × Tauri 2**

PT Analytics Next 是独立的 Warframe 利润收割者日志分析工具。采用自行编写的 Rust 类型化事件解析器与四阶段战斗状态机，独立实现日志监听、异常恢复、本地存储和 Tauri 2 前端。

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
  models.rs     # 任务数据模型
  events.rs     # 类型化日志事件解码
  engine.rs     # 解析状态机与异常隔离
  tail.rs       # 文件跟踪与完整行缓冲
  store.rs      # SQLite 持久化与去重
  main.rs       # Tauri 命令与监听线程
ui/             # 独立前端
.github/workflows/release.yml
```

目前以 Windows x64 为首发平台。首版暂未实现原应用的在线排行榜/提交、账号同步、截图及全局快捷键。计时依据可观察的游戏日志事件；真实游戏日志回归和跨平台打包需要进一步验证。数据库使用新的结构，不会覆盖原版应用数据。

## 许可证

GPL-3.0-or-later，详见 [LICENSE](LICENSE)。非 Warframe 官方产品。

## 参考项目

- [Profit-Taker-Analytics](https://github.com/nekox41/Profit-Taker-Analytics)：参考日志事件、竞速分析功能与公开界面布局。当前版本使用重新编写的核心，不包含其应用组件或旧版直接移植的 Rust 模块。
