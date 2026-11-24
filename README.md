<div align="center">

<img src="assets/logo.png" alt="PT Analytics Next" width="144" height="144">

# PT Analytics Next

**大蜘蛛竞速工具 · Rust × Tauri 2**

[![Latest Release](https://img.shields.io/github/v/release/nekox41/PT-Analytics-Next?style=flat-square&label=version&color=6eaf46)](https://github.com/nekox41/PT-Analytics-Next/releases/latest)
[![Stars](https://img.shields.io/github/stars/nekox41/PT-Analytics-Next?style=flat-square&logo=github&color=8c9bd0)](https://github.com/nekox41/PT-Analytics-Next/stargazers)
[![Downloads](https://img.shields.io/github/downloads/nekox41/PT-Analytics-Next/total?style=flat-square&label=downloads&color=6eaf46)](https://github.com/nekox41/PT-Analytics-Next/releases/latest)
[![Download](https://img.shields.io/badge/Windows%20x64-Download-6eaf46?style=flat-square)](https://github.com/nekox41/PT-Analytics-Next/releases/latest)

</div>

<div align="center">

Aameow&ChatGPT 开发的高性能、稳定且可靠的大蜘蛛竞速工具，基于 Tauri2。。
  
[下载最新 Windows 版本](https://github.com/nekox41/PT-Analytics-Next/releases/latest) · [构建状态](https://github.com/nekox41/PT-Analytics-Next/actions)

</div>

## 功能

- 自动读取日志分析大蜘蛛任务用时。
- 本地 SQLite 存储，保存所有任务记录。
- 任务生命周期：等待开始、进行中、正常完成、主动中止、意外中断、数据异常。

## 游戏内 Overlay

v0.2.0 新增实时透明卡片。进入设置 → 游戏内 Overlay，开启后默认仅在 Warframe 前台且有活动任务时显示。正常游玩时鼠标穿透，不抢占焦点；点击“调整位置与大小”进入独立预览，拖动顶部、右下角调整尺寸，完成后保存并锁定。可调整横向/纵向布局、缩放、不透明度、分项、PB 对比和隐藏规则。

复用 EE.log 状态机，不注入游戏，不读取游戏内存。阶段完成值使用日志时间，实时计时通过单调时钟短时估算；日志断开、历史回放及旧会话不会持续显示虚假计时。支持 Windows 窗口化与无边框全屏，独占全屏不保证可见。PB 差值对比最近已完成的累计分段，而非任意时刻的总时间。

## 使用

1. 下载 Windows x64 安装包或便携 EXE（需要 Microsoft Edge WebView2 Runtime；安装包支持其引导安装）。
2. 运行软件，启动 Warframe 并完成利润收割者任务；已有日志中的任务也会自动导入。
3. 如未找到日志，在设置中指定 EE.log 的绝对路径。
4. 数据保存在应用本地数据目录下的 runs.sqlite3 中。删除记录会保留去重标记，避免重启时重新导入。

若旧版 GitHub CLI 不支持 `gh release edit`，无需重新构建：确认草稿的附件均已上传后，可在 GitHub Releases 网页发布草稿。新脚本使用 REST API 发布草稿，兼容 Ubuntu 22.04 自带的旧版 gh，并在发布前检查附件及对应的源码提交。

## 开发

Windows 需要 Node.js 22、Python 3.11+、Rust stable、Visual Studio C++ Build Tools、WebView2 Runtime。

```sh
python -m pip install Pillow==11.3.0
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



## 许可证

项目采用 [MIT](LICENSE) 许可证

开发者：[Aameow](https://github.com/nekox41) & ChatGPT。

## 参考项目

- [Profit-Taker-Analytics](https://github.com/nekox41/Profit-Taker-Analytics)：参考日志事件、竞速分析功能与公开界面布局。当前版本使用重新编写的核心，不包含其应用组件或旧版直接移植的 Rust 模块。
