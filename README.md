# taskboard

macOS 向けのローカル Kanban です。ボードはデスクトップアプリ、`tb serve` の Web UI、CLI（`taskboard` / `tb`）で同じデータを共有します。

Cursor、Claude Code、Codex などの AI エージェントは、HTTP API ではなく CLI を SKILL 経由で呼び出します。

## CLI を入れる

[Homebrew](packaging/homebrew/README.md):

```bash
brew tap AI1411/taskboard
brew install taskboard
```

開発中のソースから:

```bash
cargo install --path crates/cli --locked
```

`tb` が PATH にあればそれを使い、なければ `taskboard` です。

## AI エージェントから SKILL として使う

スキル本体は [`skills/using-taskboard/SKILL.md`](skills/using-taskboard/SKILL.md) です。エージェントは実装・修正の前にカードと run を切り、終了時に `run finish` / `run fail` します。契約の全文は [`AGENTS.md`](AGENTS.md) にもあります。

`--actor` と `run start --agent` はホストに合わせます。

| ホスト | 値 |
| --- | --- |
| Cursor | `cursor` |
| Claude Code | `claude` |
| Codex | `codex` |

`TASKBOARD_ACTOR` を設定しても同じです。

### このリポジトリ

次の場所に同じスキルが置いてあります。新しいチャットを開けば読み込まれます。

| ホスト | パス |
| --- | --- |
| Cursor | `.cursor/skills/using-taskboard/` |
| Claude Code | `.claude/skills/using-taskboard/` |
| Codex | `.agents/skills/using-taskboard/` |

Cursor では [`.cursor/rules/use-taskboard.mdc`](.cursor/rules/use-taskboard.mdc) も常時適用です。

### 別のリポジトリ、またはすべてのプロジェクト

1. このリポジトリを clone し、CLI を PATH に入れる。
2. スキルディレクトリをリンク（またはコピー）する。`TASKBOARD` は clone 先です。

| ホスト | このプロジェクトだけ | すべてのプロジェクト |
| --- | --- | --- |
| Cursor | `.cursor/skills/using-taskboard` | `~/.cursor/skills/using-taskboard` |
| Claude Code | `.claude/skills/using-taskboard` | `~/.claude/skills/using-taskboard` |
| Codex | `.agents/skills/using-taskboard` | `~/.agents/skills/using-taskboard` |

```bash
TASKBOARD="$HOME/dev/taskboard"

# Cursor — 今のリポジトリ
mkdir -p .cursor/skills
ln -snf "$TASKBOARD/skills/using-taskboard" .cursor/skills/using-taskboard

# Claude Code — すべてのプロジェクト
mkdir -p ~/.claude/skills
ln -snf "$TASKBOARD/skills/using-taskboard" ~/.claude/skills/using-taskboard

# Codex — すべてのプロジェクト
mkdir -p ~/.agents/skills
ln -snf "$TASKBOARD/skills/using-taskboard" ~/.agents/skills/using-taskboard
```

コピーする場合:

```bash
cp -R "$TASKBOARD/skills/using-taskboard" .cursor/skills/using-taskboard
```

3. 対象リポジトリに [`AGENTS.md`](AGENTS.md) を置くか、同じ契約を追記する。`--actor cursor` はホスト名に書き換える。
4. **新しいチャット / セッションを開始する。** スキルは起動中のセッションには入らないことがあります。

### 動きの確認

1. `tb serve --open`（またはこのリポジトリで `task start`）でボードを開く。
2. エージェントに、小さな実装をして Taskboard に記録するよう頼む。
3. カードが In Progress になり、run が始まり、終了後に In Review または Done へ動くことを確認する。

質問だけのやり取りではボードを更新しなくて構いません。
