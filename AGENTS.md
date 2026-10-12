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

## 同步上游时的额外规则（必遵守）

1. **同步上游前，必须保证当前代码可回滚**
   - 在执行 `git fetch upstream`、`git merge upstream/...`、`git rebase upstream/...` 之前，必须先确认当前状态可回退。
   - 至少满足以下之一：
     - 当前工作树干净，且当前提交已经存在于本地分支历史中；
     - 或者先创建一个明确命名的备份分支/备份标签，例如：
       - `backup/<branch>-pre-upstream-sync-<timestamp>`
   - 如果存在未提交修改，必须先处理（提交、stash，或经用户明确同意后放弃），不能直接开始同步。

2. **遇到冲突时，默认优先保留本地实现**
   - 这里的“本地实现”指**当前未同步上游之前，本分支已经存在的实现**。
   - 只有在能够明确证明上游实现更适合、且不会破坏本分支目标时，才允许偏向上游版本。
   - 如果选择了上游实现或混合方案，必须在最终汇报中明确说明原因，而不是模糊带过。

3. **解决完冲突后，必须向用户说明处理了哪些冲突**
   - 至少说明：哪些文件发生了冲突；每个冲突最终是保留本地、保留上游，还是手工合并；如果某处没有遵循“优先保留本地实现”，必须说明理由。
   - 不要只说“已解决冲突”，必须给出文件级别的具体说明。

4. **同步完成后，要做一次结果审计**
   - 核对本分支关键改动是否仍然存在（例如用户明确点名的 i18n/汉化改动）。
   - 核对是否已经真正合入目标上游提交，而不是只停留在 `fetch` 或半完成 merge 状态。
   - 如果仍处于 merge/rebase 未完成状态，不能向用户宣称“同步完成”。

5. **同步完成后，必须补充中文 i18n**
   - 每次合入上游后，必须审计新增/变更的用户可见 UI 文本。
   - 对所有新增用户可见文本补充 `app/src/i18n/zh_cn.json`。
   - 对不会自动经过 `Text` / `FormattedTextElement::from_str` 翻译钩子的动态字符串、`FormattedTextFragment`、菜单项等，必须改调用点使用 `crate::i18n::ui_str` / `ui_text`，或补充保守的动态翻译逻辑。
   - 最终汇报中说明本次 i18n 审计范围，不能只说“已同步上游”。

## 上游开发参考（供参考）

> 说明：上游 `AGENTS.md` 中的本地构建/测试命令（`cargo build`、`cargo run`、`./script/run` 等）在本分支**不适用**——本仓库禁止本地编译与打包，统一交给 GitHub Actions。以下仅保留上游的架构与开发约定参考内容。

## Architecture Overview

This is a Rust-based terminal emulator with a custom UI framework called **WarpUI**. It has **two front-ends** that share a common core.

### Front-ends: GUI and TUI

Warp has two front-ends that share the `warp_core`/`warpui` Entity/model core (App/Entity/`AppContext`, actions, `Appearance`, `FeatureFlag`, telemetry, logging) but differ in UI framework, rendering, input, and verification:
- **GUI desktop app** — the `app/` crate on the WarpUI pixel/GPU framework (`warpui`, `crates/warpui_core`): `Element`/`View` layout, GPU/WGSL rendering, mouse input, `.app` bundles. Run with `cargo run` / `./script/run`; verify visually with `computer_use` or the real-display integration framework (`crates/integration`).
- **Headless TUI** — the `crates/warp_tui` crate: a console app (run with `./script/run-tui`; no `.app`/GPU) rendered with a parallel cell-grid element library at `crates/warpui_core/src/elements/tui` (the `TuiElement` trait), behind the `tui` cargo feature. Verify by running it in a real terminal and observing output; test with render-to-lines unit tests.

**Skill convention:** a skill specific to one front-end says so in its name and/or description (e.g. `gui-ui-guidelines` / `gui-integration-test` are GUI-only; `tui-ui-guidelines`, `tui-testing`, and `tui-verify-change` are TUI-specific). Skills with no front-end call-out are surface-agnostic and apply to both. For TUI work prefer the `tui-*` skills and ignore GUI-only ones — and vice versa.

### Key Components

**Shared UI core** (`crates/warpui`, `crates/warpui_core`) — used by **both** front-ends:
- Entity-Component-Handle pattern: a global `App` object owns all views/models (entities); views hold `ViewHandle<T>` references to other views; `AppContext` provides temporary access to handles during render/events.
- Actions system for event handling.
- `crates/warpui_core` also hosts the TUI cell-grid element library under `src/elements/tui` (behind the `tui` feature).

**GUI rendering** (WarpUI GUI elements — GUI-specific):
- `Element`s describe visual layout (Flutter-inspired), rendered on the GPU (WGSL).
- Mouse input uses `MouseStateHandle`: create it once during construction and reference/clone it wherever mouse input is tracked. An inline `MouseStateHandle::default()` while rendering means no mouse interactions work. (The TUI's hover/click elements — `TuiHoverable`, `tui_collapsible` — also build on `MouseStateHandle`, so the same ownership rule applies there.)

**TUI rendering** (`crates/warp_tui` + `crates/warpui_core/src/elements/tui` — TUI-specific):
- Headless console front-end. The `TuiElement` trait lays out and paints into a cell-grid `TuiBuffer`; crossterm input is converted to `TuiEvent`. No GPU/WGSL, pixel geometry, or `.app` bundle.

**Main app / shared surfaces** (`app/`) — the GUI desktop app plus feature surfaces the TUI reuses:
- Terminal emulation and shell management (`terminal/`)
- AI integration including Agent Mode (`ai/`)
- Cloud synchronization and Drive features (`drive/`)
- Authentication and user management (`auth/`)
- Settings and preferences (`settings/`)
- Workspace and session management (`workspace/`)

**Core Libraries**:
- `crates/warp_core/` - Core utilities and platform abstractions (shared)
- `crates/warp_tui/` - Headless TUI front-end
- `crates/editor/` - Text editing functionality
- `crates/warpui/` and `crates/warpui_core/` - Custom UI framework (shared core plus the GUI and TUI element libraries)
- `crates/ipc/` - Inter-process communication
- `crates/graphql/` - GraphQL client and schema

### Key Architectural Patterns

1. **Entity-Handle System**: Views reference other views via handles, not direct ownership
2. **Modular Structure**: Workspace contains multiple workspace configurations, each with terminals, notebooks, etc.
3. **Cross-Platform**: Native implementations for macOS, Windows, Linux, plus WASM target
4. **AI Integration**: Built-in AI assistant with context awareness and codebase indexing
5. **Cloud Sync**: Objects can be synchronized across devices via Warp Drive

### Development Guidelines

**Workspace Structure**:
- This is a Cargo workspace with 60+ member crates
- Main binary is in `app/`, UI framework in `crates/warpui/`
- Platform-specific code is conditionally compiled
- Integration tests are in `crates/integration/`

**Coding Style Preferences**:
- Avoid unnecessary type annotations, especially in closure params.
- Avoid using too many Rust path qualifiers and use imports for concision. Place import statements at the top of the file as per convention.
  An exception to this is inside cfg-guarded code branches. In those cases, you can either embed the import into the relevant scope or just use an absolute path for one-offs.
- If a function takes a context parameter (`AppContext`, `ViewContext`, or `ModelContext`), it should be named `ctx` and go last. The one exception is for
  functions that take a closure parameter, in which case the closure should be last.
- Always remove unused parameters completely rather than prefixing them with `_`. Update the function signature and all call sites accordingly.
- Prefer inline format arguments in macros like `println!`, `eprintln!`, and `format!` (for example, `eprintln!("{message}")` instead of `eprintln!("{}", message)`) to satisfy Clippy's `uninlined_format_args` lint.
- Do not pass `Itertools::format` results directly to logging macros (`log::*`, `safe_*`, etc.). `Itertools::format` produces a single-use formatter, while logging implementations may format a message more than once. Use a reusable `String` such as `iter.join(", ")` for logging arguments instead. Direct use in `format!` or `write!` is fine.
- When adding a toggleable setting, also add the matching Command Palette enable/disable entry and any required context flags so the setting is discoverable outside Settings.

**Comments**:
Comments have a cost. They carry a maintenance burden, because they must be kept in sync
with the code they describe. It is tempting to assume that more comments is always better,
but be judicious about when a comment is actually necessary because the code cannot speak
for itself.
- **Minimalist Comments**: Assume the reader is a Senior Software Engineer. Never comment
  to explain WHAT or HOW code works if self-documenting names accomplish that.
- **Strictly "Why" Only**: Reserve inline comments strictly for non-obvious business
  rationale, workarounds for third-party bugs, complex algorithms, unidiomatic code, or
  unexpected edge cases.
- **No Line-by-Line Narrations**: Never add comments restating the syntax (e.g., omit
  `# Initialize array`, `# Loop over users`).
- **Clean Docstrings**: Keep doc comments concise. Document public APIs, arguments, types,
  and returns. Do not narrate the method's internal implementation steps.
- **Single-source of documentation**: For items/members that have a doc comment explaining
  their purpose, you do not need to repeat that explanation anywhere else. A good example
  is a float const specifying an amount of spacing. You may use a doc comment on the
  declaration if necessary, but do not repeat that where the const is *referenced*. Another
  example is function call sites. Function doc comments explain what they do. Do not repeat
  the explanation at the call site.
- **Container docs describe the whole, member docs describe the parts**: A field's, variant's,
  or parameter's own doc comment is where that member gets explained. A container's item-level
  doc comment (struct, enum, trait) describes the item as a whole and must not enumerate or
  re-explain its members. Do not name members in the container's doc comment just to describe
  them, and do not restate in a member's doc comment what the container's doc comment already
  said. Behavior genuinely shared by several members belongs in exactly one place, not both.
- **Don't enumerate function call sites in doc comments**: Function doc comments should
  document their behavior and NOT their callers, e.g. it should never say things like,
  "this is used by [certain callers]" or "this is used when...".
- **No "transformation comments"**: Do not add comments that explain *your edits*. Comments
  only need explain the *current state* of the code. Explanations of edits belong in pull
  request comments instead. You shouldn't add comments with phrases like, "this used to do
  so-and-so".
- Do not remove existing comments when making unrelated changes. Only remove or modify a
  comment if the logic it describes has changed.
- The formatter (`./script/format`) is configured with a `max_width` (max line length) of
  100. Flow (reflow) comment line-wrapping to fill that full width rather than wrapping
  early at a narrower column, so comments span as few lines as possible.

**Terminal Model Locking**:
- Be extremely careful when calling `model.lock()` on the terminal model (`TerminalModel`). Acquiring multiple locks on the same model from different call sites can cause a deadlock, resulting in a UI freeze (beach ball on macOS).
- Before adding a new `model.lock()` call, verify that no caller in the current call stack already holds the lock.
- Prefer passing already-locked model references down the call stack rather than acquiring new locks.
- If you must lock the model, keep the lock scope as short as possible and avoid calling other functions that might also attempt to lock.

**Testing**:
- Use `cargo nextest` for parallel test execution
- Integration tests use the custom framework in `crates/integration/` — this is **GUI-only**. TUI elements/screens are covered by render-to-lines unit tests instead (see the `tui-testing` skill).
- Follow the Implementation Validation Order above; do not add a full presubmit run after targeted tests unless it was explicitly required.
- Unit tests should be placed in separate files using the naming convention `${filename}_tests.rs` or `mod_test.rs`
- Test files should be included at the end of their corresponding module with:
  ```rust
  #[cfg(test)]
  #[path = "filename_tests.rs"]  // or "mod_test.rs"
  mod tests;
  ```

**Pull Request Workflow**:
- Follow the Implementation Validation Order before opening a PR or pushing a code update. Do not repeat validation when the candidate has not changed.
- CI is the broad cross-platform and workspace gate. Push once the targeted tests and lint checks pass and the formatter has run; address a later CI failure as a new revision.
- Do not create public pull requests or public issues that disclose a non-public security vulnerability. Refer users to `SECURITY.md` for the proper disclosure methods instead.
 - When opening PRs, use the PR template at `.github/pull_request_template.md`
 - Add changelog entries when appropriate using the format at the bottom of the PR template. Use the following prefixes (without the `{{}}` brackets):
   - `CHANGELOG-NEW-FEATURE:` for new, relatively sizable features (use sparingly - these may get marketing/docs)
   - `CHANGELOG-IMPROVEMENT:` for new functionality of existing features
   - `CHANGELOG-BUG-FIX:` for fixes related to known bugs or regressions
   - `CHANGELOG-IMAGE:` for GCP-hosted image URLs
   - Leave changelog lines blank or remove them if no changelog entry is needed

**Database**:
- Uses Diesel ORM with SQLite
- Migrations in `crates/persistence/migrations/`
- Schema defined in `crates/persistence/src/schema.rs`

**GraphQL**:
- Schema and client code generation from `crates/warp_graphql_schema/api/schema.graphql`
- TypeScript types generated for frontend integration

### Feature Flags

Warp uses compile-time feature flags with a small runtime plumbing layer.

How to add a feature flag:
- Add a new variant to `warp_core/src/features.rs` in the `FeatureFlag` enum
- (Optional) Enable it by default for dogfood builds by listing it in `DOGFOOD_FLAGS`
- Gate code paths with `FeatureFlag::YourFlag.is_enabled()`
- For preview or release rollout, add to `PREVIEW_FLAGS` or `RELEASE_FLAGS` respectively (as appropriate)

Best practices:
- **Prefer runtime checks over cfg directives**: Prefer `FeatureFlag::YourFlag.is_enabled()` over `#[cfg(...)]` compile-time directives so flags can be toggled without recompilation and are easier to clean up later. Use `#[cfg(...)]` only when the code cannot compile without them (for example, platform-specific code or dependencies that do not exist when the feature is disabled).
- Keep flags high-level and product-focused rather than per-call-site
- Remove the flag and dead branches after launch has stabilized
- For UI sections that expose a new feature, hide the UI behind the same flag

Example:
```rust
#[derive(Sequence)]
pub enum FeatureFlag {
    YourNewFeature,
}

// Default-on for dogfood builds
pub const DOGFOOD_FLAGS: &[FeatureFlag] = &[
    FeatureFlag::YourNewFeature,
];

// Use in code
if FeatureFlag::YourNewFeature.is_enabled() {
    // gated behavior
}
```

### Exhaustive Matching

When adding/editing match statements, avoid using the wildcard _ when at all possible. Exhaustive matching is helpful for ensuring that all variants are handled, especially when adding new variants to enums in the future.
