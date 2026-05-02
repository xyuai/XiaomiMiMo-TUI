# Slash command audit

Audit started: 2026-05-02 15:05 +08:00.

All public slash commands are registered in `crates/tui/src/commands/mod.rs`.
The registry is covered by unit tests that verify:

- command names are unique;
- aliases resolve to their owning command;
- every command has a conservative smoke-test invocation; and
- those smoke invocations dispatch without panicking.

## Commands tested

| Command | Smoke invocation | Notes |
| --- | --- | --- |
| `/help` | `/help clear` | help topic |
| `/clear` | `/clear` | clears in-memory test app state |
| `/exit` | `/exit` | emits quit action |
| `/model` | `/model mimo-v2-flash` | model parser |
| `/models` | `/models` | emits fetch-models action |
| `/provider` | `/provider xiaomimimo` | provider parser |
| `/queue` | `/queue list` | non-mutating queue path |
| `/subagents` | `/subagents` | emits list action |
| `/links` | `/links` | dashboard/docs text |
| `/home` | `/home` | dashboard text |
| `/note` | `/note slash command smoke test` | writes inside temp workspace |
| `/attach` | `/attach missing-smoke-test-image.png` | validates error path |
| `/task` | `/task list` | emits task-list action |
| `/jobs` | `/jobs list` | emits shell-job list action |
| `/mcp` | `/mcp status` | emits MCP status action |
| `/save` | `/save slash_command_smoke_session.json` | writes inside temp workspace |
| `/sessions` | `/sessions` | opens session picker |
| `/load` | `/load missing-smoke-test-session.json` | validates error path |
| `/compact` | `/compact` | emits compact action |
| `/context` | `/context` | opens context inspector |
| `/cycles` | `/cycles` | lists/empty-state path |
| `/cycle` | `/cycle 1` | validates missing cycle |
| `/recall` | `/recall` | validates usage; async archive path is covered by tool tests |
| `/export` | `/export` | writes inside temp workspace |
| `/config` | `/config` | opens config view |
| `/yolo` | `/yolo` | mode switch |
| `/agent` | `/agent` | mode switch |
| `/plan` | `/plan` | mode switch |
| `/trust` | `/trust status` | non-mutating trust status |
| `/logout` | `/logout` | covered by env-guarded unit test |
| `/tokens` | `/tokens` | token summary |
| `/system` | `/system` | system prompt summary |
| `/undo` | `/undo` | empty-state path |
| `/retry` | `/retry` | no-previous-request path |
| `/init` | `/init` | writes `AGENTS.md` inside temp workspace |
| `/settings` | `/settings` | loads default/settings file |
| `/statusline` | `/statusline` | opens status picker |
| `/skills` | `/skills` | lists temp skill dir |
| `/skill` | `/skill missing-smoke-test-skill` | validates not-found path |
| `/review` | `/review smoke-target` | validates missing review skill or emits send action |
| `/restore` | `/restore` | lists empty snapshots |
| `/rlm` | long `/rlm ...` prompt | emits RLM action |
| `/cost` | `/cost` | token summary |

## Fixes made

- Removed the duplicate `/context` registry entry. The remaining entry keeps the
  `/ctx` alias and opens the context inspector.
- Added registry/smoke tests so future commands must remain discoverable and
  dispatchable.
- Fixed `/save` with a relative path to write inside the active workspace.
- Fixed `/export` with a relative/default path to write inside the active
  workspace and create parent directories when needed.
- Fixed `/attach ~/...` expansion to use the platform home resolver instead of
  only the `HOME` environment variable, improving Windows behavior.
