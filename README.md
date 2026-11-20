# XASM Rust + Community

这个工作区把两套实现分开：

- [rust/](rust/)：Rust 重写的 XASM 编译器、VM 和 XPL 语言，可独立作为 git 仓库 push。
- [XASM_Community_Rebuild/](XASM_Community_Rebuild/)：手写的 community rebuild，保留自己的 git 历史和 remote。

原版 XASM1 的 dispatch fallthrough bug 的最小复现在 [rust/repro/](rust/repro/)。

Rust 项目入口和文档见 [rust/README.md](rust/README.md)。
