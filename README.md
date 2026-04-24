<div align="center">

# QIMENG · SCUT Survival Manual Browser

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-GPL%20v2-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/Kozmosa/qimeng-scut?include_prereleases)](https://github.com/Kozmosa/qimeng-scut/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/Kozmosa/qimeng-scut/release.yml?label=CI)](https://github.com/Kozmosa/qimeng-scut/actions)

**🎓 在终端里优雅地浏览华南理工大学生存手册**

[📥 下载最新版](https://github.com/Kozmosa/qimeng-scut/releases/latest) · [🐛 报告问题](https://github.com/Kozmosa/qimeng-scut/issues) · [📖 使用指南](#使用指南)

</div>

---

## ✨ 特性

- 🖥️ **终端原生体验** — 基于 [Ratatui](https://github.com/ratatui/ratatui) 构建，流畅的 TUI 界面
- 📚 **Markdown 渲染** — 支持标题、代码块、列表等富文本内容展示
- 🗂️ **三栏式导航** — 分区 → 条目 → 内容，方向键/Enter 快速跳转
- 🔄 **双栏阅读模式** — 按 `t` 键切换单双栏布局，宽屏阅读更高效
- 🎨 **ASCII 艺术 Banner** — 启动时展示 Figlet 风格的开屏画面
- 🚀 **跨平台支持** — Windows / Linux / macOS，x86_64 & ARM64

## 📦 安装

### 预编译二进制

从 [Releases](https://github.com/Kozmosa/qimeng-scut/releases) 页面下载对应平台的压缩包：

| 平台 | AMD64 | ARM64 |
|:---:|:---:|:---:|
| **Linux** | `tar.gz` | `tar.gz` |
| **macOS** | `tar.gz` | `tar.gz` |
| **Windows** | `zip` | `zip` |

解压后将 `qimeng-scut` 放到 `PATH` 中即可使用。

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/Kozmosa/qimeng-scut.git
cd qimeng-scut

# 编译 Release 版本
cargo build --release

# 运行
./target/release/qimeng-scut
```

> 需要 Rust 1.70+，可通过 [rustup](https://rustup.rs/) 安装。

## 🚀 使用指南

### 启动程序

```bash
qimeng-scut
```

### 首页命令

| 命令 | 说明 |
|:---:|:---|
| `manual` | 进入手册浏览模式 |
| `help` | 显示帮助信息 |
| `exit` / `q` | 退出程序 |

### 手册浏览模式快捷键

| 按键 | 功能 |
|:---:|:---|
| `←` `→` | 在分区 / 条目 / 内容三栏间切换焦点 |
| `↑` `↓` | 上下滚动或浏览列表 |
| `Enter` | 确认选择（打开分区或文档） |
| `t` | 切换单栏 / 双栏阅读模式 |
| `Esc` | 返回首页 |
| `q` | 退出程序 |

### 手册仓库结构

程序需要一个包含 `docs/` 子目录的本地路径作为手册仓库：

```
my-manual/
├── docs/
│   ├── README.md          # 首页 / 顶层文档
│   ├── get-started.md     # 更多顶层文档
│   ├── health/            # 分区目录 → 对应一个分区
│   │   ├── medical.md
│   │   └── nested/
│   │       └── alive.md
│   └── others/
│       └── app.md
```

- `docs/` 下的 `.md` 文件归入「首页 / 顶层」分区
- 每个子目录自动成为一个独立分区
- 支持多级嵌套目录，隐藏文件和空目录会被忽略

## 🖼️ 界面预览

```
+-----------------------------------------------------------------------------+
|  +=======================================================================+  |
|  |        QIMENG  ·  SCUT Survival Manual Browser                        |  |
|  +=======================================================================+  |
|                                                                             |
|  +--------------+--------------+----------------------------------------+  |
|  | Sections     | Entries      | Content                                |  |
|  |              |              |                                        |  |
|  | > Top Level  | > Home       | # Welcome to SCUT Survival Manual      |  |
|  |   health     |   Getting    |                                        |  |
|  |   others     |   Started    | This is a survival guide for SCUT      |  |
|  |              |              | students...                            |  |
|  |              |              |                                        |  |
|  |              |              | ## Quick Start                         |  |
|  |              |              |                                        |  |
|  +--------------+--------------+----------------------------------------+  |
|                                                                             |
|  [Home] Type `manual` to enter manual browsing mode.                        |
+-----------------------------------------------------------------------------+
```

## 🛠️ 技术栈

- [Rust](https://www.rust-lang.org/) — 系统级性能与内存安全
- [Ratatui](https://github.com/ratatui/ratatui) — 现代 Rust TUI 框架
- [Crossterm](https://github.com/crossterm-rs/crossterm) — 跨平台终端控制
- [tui-markdown](https://crates.io/crates/tui-markdown) — Markdown → TUI 渲染
- [figlet-rs](https://crates.io/crates/figlet-rs) — ASCII 艺术字生成
- [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) — Markdown 解析器

## 🤝 贡献

欢迎 Issue 和 PR！

```bash
# 运行测试
cargo test

# 检查代码
cargo clippy
cargo fmt
```

## 📄 许可

本项目基于 [GPL v2 License](LICENSE) 开源。

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Kozmosa/qimeng-scut&type=Date)](https://star-history.com/#Kozmosa/qimeng-scut&Date)

---

<div align="center">

Made with ❤️ by <a href="https://github.com/Kozmosa">Kozmosa</a>

</div>
