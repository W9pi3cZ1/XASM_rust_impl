# XASM Rust + Community

这个工作区把两套实现分开：

- [rust/](rust/)：Rust 重写的 XASM 编译器、VM 和 XPL 语言，可独立作为 git 仓库 push。
- `XASM_Community_Rebuild/`：手写的 community rebuild，作为 git submodule 引入。

原版 XASM1 的 dispatch fallthrough bug 的最小复现在 [rust/repro/](rust/repro/)。

Rust 项目入口和文档见 [rust/README.md](rust/README.md)。

```bash
git clone --recurse-submodules https://github.com/W9pi3cZ1/XASM_rust_impl.git
cd XASM_rust_impl/rust
cargo test --workspace
```

已经 clone 但没初始化 submodule 的话：

```bash
git submodule update --init --recursive
```

测试会从 `rust/` 找到 `../XASM_Community_Rebuild`。
