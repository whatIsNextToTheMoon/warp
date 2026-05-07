# TODO

- [ ] 修复异常退出后 Warp 自己的界面/会话历史可能为空或回退到较旧状态的问题。
  - 现象：卡退、崩溃或强制退出后，Warp 内部记录的历史可能丢失或不是最近状态；这不是 shell 的 `zsh_history`。
  - 初步排查方向：确认 UI/session 历史写入 SQLite/WAL 的时机、写入队列 flush、异常退出恢复路径，以及退出前最后一批 block/session 状态是否可靠落盘。
