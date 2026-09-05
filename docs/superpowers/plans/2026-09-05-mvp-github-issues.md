# Local Taskboard MVP — GitHub Issue Map

既存の詳細設計と 3 本の実装プランを、レビュー可能な単位の GitHub issue に落とした。実装時は各 issue が指すプランタスクを TDD で実行する。

**Epic:** [#25 Local Taskboard MVP](https://github.com/AI1411/taskboard/issues/25)  
**Milestone:** [MVP](https://github.com/AI1411/taskboard/milestone/1)

仕様:

- `docs/superpowers/specs/2026-09-05-local-taskboard-design.md`
- `docs/superpowers/specs/2026-09-05-local-taskboard-detailed-design.md`

## 着手順

```text
#3 → #4 → #5 → #6
         ↘ #7 → #8 → #9 → #10 → #11 → #12 → #13
                                           ↘ #14 → #15 → #16 → #17 → #18
                                                ↘ #19 → #20 → #21 → #22
#23 は #3 の直後から並行可
#24 は macOS CLI バイナリのあと（3 プラン対象外）
```

最初に着手する issue は [#3](https://github.com/AI1411/taskboard/issues/3)。

## Plan 1 — Core, SQLite, CLI

プラン: `docs/superpowers/plans/2026-09-05-taskboard-core-cli.md`

| Issue | Plan task | 内容 |
| --- | --- | --- |
| [#3](https://github.com/AI1411/taskboard/issues/3) | Task 1 | Workspace and crate stubs |
| [#4](https://github.com/AI1411/taskboard/issues/4) | Task 2 | IDs, enums, slug, derived idle status |
| [#5](https://github.com/AI1411/taskboard/issues/5) | Task 3 | Validation helpers and entity structs |
| [#6](https://github.com/AI1411/taskboard/issues/6) | Task 4 | Column display order |
| [#7](https://github.com/AI1411/taskboard/issues/7) | Task 5 | Application errors and Store trait |
| [#8](https://github.com/AI1411/taskboard/issues/8) | Task 6 | SQLite open, data dir, migration 0001 |
| [#9](https://github.com/AI1411/taskboard/issues/9) | Task 7 | Project use cases |
| [#10](https://github.com/AI1411/taskboard/issues/10) | Task 8 | Task create, list, show, move, reorder, urgent |
| [#11](https://github.com/AI1411/taskboard/issues/11) | Task 9 | Notes, links, runs |
| [#12](https://github.com/AI1411/taskboard/issues/12) | Task 10 | Revisions, trash, purge, undo, backup |
| [#13](https://github.com/AI1411/taskboard/issues/13) | Task 11 | CLI clap, JSON, human output, e2e |

Plan 1 完了時: `taskboard` / `tb` が一時データディレクトリで US-01..13, US-15 を満たす。`tb serve` は未実装スタブ。

## Plan 2 — Local HTTP API and web UI

プラン: `docs/superpowers/plans/2026-09-05-taskboard-api-web.md`  
Blocked by: Plan 1 (`#13`)

| Issue | Plan task | 内容 |
| --- | --- | --- |
| [#14](https://github.com/AI1411/taskboard/issues/14) | Task 1 | Axum server, session, Origin |
| [#15](https://github.com/AI1411/taskboard/issues/15) | Task 2 | HTTP routes matching App use cases |
| [#16](https://github.com/AI1411/taskboard/issues/16) | Task 3 | TypeScript Transport and HttpTransport |
| [#17](https://github.com/AI1411/taskboard/issues/17) | Task 4 | UI board, inspector, keyboard, Native Glass |
| [#18](https://github.com/AI1411/taskboard/issues/18) | Task 5 | Vite web app, poll, serve static, e2e |

Plan 2 完了時: `tb serve` で localhost ボードが動き、CLI 変更が 2 秒以内に見える（US-14）。

## Plan 3 — Tauri desktop

プラン: `docs/superpowers/plans/2026-09-05-taskboard-desktop.md`  
Blocked by: Plan 1 `App` と Plan 2 共有 UI

| Issue | Plan task | 内容 |
| --- | --- | --- |
| [#19](https://github.com/AI1411/taskboard/issues/19) | Task 1 | TauriTransport |
| [#20](https://github.com/AI1411/taskboard/issues/20) | Task 2 | Tauri crate and commands |
| [#21](https://github.com/AI1411/taskboard/issues/21) | Task 3 | Desktop frontend entry and smoke |
| [#22](https://github.com/AI1411/taskboard/issues/22) | Task 4 | Undo, CLI refresh, packaging notes |

Plan 3 完了時: デスクトップは HTTP を使わず同じ SQLite を操作する。`.dmg` 実体は macOS builder。

## 横断 / リリース

| Issue | 内容 | 備考 |
| --- | --- | --- |
| [#23](https://github.com/AI1411/taskboard/issues/23) | GitHub Actions（fmt, clippy, cargo test, pnpm test） | `#3` の直後から並行可 |
| [#24](https://github.com/AI1411/taskboard/issues/24) | Homebrew tap `AI1411/taskboard` | 3 プラン対象外。aarch64-apple-darwin バイナリのあと |

## ラベル

- `epic` — トラッキング
- `plan:core-cli` / `plan:api-web` / `plan:desktop` — 実装プラン
- `plan:release` — 配布
- `enhancement` — GitHub 標準

## 粒度の決め方

各 issue は実装プランの 1 タスクに対応する。プラン内の 2–5 分ステップ（失敗するテストを書く、実装する、コミットする）は issue にせず、プラン本文に残した。issue は「レビュー可能な 1 ゲート」であり、ステップは実行時の手順である。
