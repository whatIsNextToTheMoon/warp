# AGENTS.md

本文件用于约束/指导自动化代理在本仓库中的行为（尤其是 CI 产物构建方式）。

## 最重要规则（必遵守）

1. **禁止本地编译/本地打包**
   - 不要在本地/容器内运行任何会触发编译或打包的命令，例如（不限于）：
     - `cargo build` / `cargo test` / `cargo run`
     - `./script/linux/bundle ...`
     - `./script/bundle ...`
     - 任何会间接调用 Rust 编译的脚本/命令
   - 允许做的事情：代码修改、文本处理、`rg`/`sed`/`jq`/`python` 这类**不触发编译**的离线处理与校对。

2. **构建统一交给 GitHub Actions**
   - 当完成代码修改后：`git commit` + `git push origin build-appimage`（或当前工作分支）触发 GitHub Actions 编译/打包。
   - 不要为了“验证能不能过编译”而在本地编译；以 Actions 结果为准。

3. **默认使用简体中文沟通**
   - 除非用户明确要求英文，否则所有解释使用简体中文；命令/代码标识符/日志保持原样。

## 提交与 CI 约定

- 提交信息尽量清晰：`i18n(zh-CN): ...`、`ci(appimage): ...`、`fix: ...`。
- 所有与产物相关的改动（AppImage / Arch 包等）都必须确保对应的 GitHub Actions 工作流会被触发并产出 artifact。

