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
- 任务参与玩家名单与自己的身份标记；根据本地登录/名称变更识别自己，根据任务期间的仲裁及配装加载记录识别参与者，支持按玩家搜索。旧记录在原日志仍可回放时补全名单。没有身份依据时不猜测。
- 任务生命周期：等待开始、进行中、正常完成、主动中止、意外中断、数据异常。历史默认仅显示正常完成任务。
- 主动中止按明确退出事件结算；意外中断按最后可信战斗事件计算耗时下限，保留已完成阶段和未完成阶段的已确认分段。
- 活动任务检查点独立存储；软件重启优先回放原日志，原会话无法继续时才归档为中断，后续完整记录可覆盖中断快照。仅正常完成且数据有效的任务参与统计。
- 平均耗时、PB、任务耗时趋势图；深色和浅色主题。
- 日志监听状态、同步进度、异常行计数及读取错误。
- 不需要原版 Flutter 应用、在线账号或远程服务。

- 成绩卡片按阶段显示实际机制（第二阶段仅破腿、本体；第四阶段无信标），同时显示单阶段和累计耗时。
- 一键复制透明背景成绩图片（适配当前主题与布局）、PTA 风格英文文本战报；与上一场有效记录比较耗时差值。

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

目前以 Windows x64 为首发平台。首版暂未实现原应用的在线排行榜/提交、账号同步、截图及全局快捷键。计时依据可观察的游戏日志事件；已加入 2026-11-24 真实日志的脱敏事件回归样本（一次中止、一次完整任务），覆盖实际多行任务配置及四阶段计时；多人、异常信标和跨平台仍需进一步验证。数据库使用新的结构，不会覆盖原版应用数据。

## 许可证

GPL-3.0-or-later，详见 [LICENSE](LICENSE)。非 Warframe 官方产品。

## 参考项目

- [Profit-Taker-Analytics](https://github.com/nekox41/Profit-Taker-Analytics)：参考日志事件、竞速分析功能与公开界面布局。当前版本使用重新编写的核心，不包含其应用组件或旧版直接移植的 Rust 模块。
