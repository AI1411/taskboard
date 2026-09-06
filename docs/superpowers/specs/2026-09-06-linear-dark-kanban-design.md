# Linear 風ダーク Kanban

日付: 2026-09-06  
対象: localhost web UI と、同じ `packages/ui` を使う Tauri デスクトップ  
親スペック: `docs/superpowers/specs/2026-09-05-local-taskboard-design.md`

この文書は、親スペックの **§6 Visual design（Native Glass）** を UI 表面について置き換える。ドメイン、4列、CLI、キーボード契約は親のまま。

## 1. 要約

Taskboard のボードを Linear のボード言語に寄せる。面はほぼ黒、境界はヘアライン、字は Inter、アクセントのインディゴは選択と主ボタンだけ。カード詳細は右カラムではなく、全幅ボードの上に乗るオーバーレイにする。

Issue リスト、コマンドパレット、Trash 画面、URL への状態保存は作らない。Linear のクローンではない。

## 2. 決まったこと

| 項目 | 決定 |
|------|------|
| 面 | Linear dark（ほぼ黒） |
| ホーム | 4列 Kanban のまま |
| 詳細 | 右から被さるオーバーレイ |
| 深さ | Linear のボード言語（トークン＋カード＋オーバーレイ）。製品クローンではない |

## 3. 面とタイポ

| 役割 | 値 |
|------|-----|
| ワークスペース | `#08090a` |
| サイドバー | `#0c0d0e` |
| カラム／パネル | `#101113` |
| カード | `#16171a` |
| ヘアライン | `#1b1c1f` / `#23242a` |
| 本文 | `#e8e8ec` |
| ミュート | `#8a8f98` |
| アクセント | `#5e6ad2`（主ボタンと選択ボーダーだけ） |
| Urgent | 親スペックどおりオレンジレッド |
| 完了 / 失敗 | 親スペックの緑 / 赤 |

- フォント: Inter。フォールバックは `"SF Pro Text", system-ui, sans-serif`。
- `html` に `color-scheme: dark`。
- ガラス、ラベンダーのボード、カードのドロップシャドウは使わない。
- 角丸は 6–8px。大きく膨らませない。

トークンは `packages/ui/src/theme.css` の CSS 変数に集約する。コンポーネントへ hex を散らさない。

## 4. ボードとカード

- 列: Todo / In Progress / In Review / Done。ラベルと件数はミュート。
- カード: タイトル、その下に `TASK-n`（tabular-nums）とステータスピル。影なし。選択中は 1px のインディゴボーダー。
- 空列: 破線スロットのままでよい。列ごとの「追加」ボタンは今回やらない。ツールバーの New task が入口。
- 未選択時はインスペクタを出さない。ボードはビューポート幅いっぱい。

## 5. オーバーレイ

- 幅およそ 360px。ボードは縮まない。パネルが右から乗る。
- 開く: カード選択。閉じる: Escape、パネル外クリック、選択解除。別カードを選ぶと中身だけ差し替え。
- 中身: タイトル、`TASK-n`、列、Urgent、ノート、Links / Runs / Activity、削除確認（Delete → Move to Trash / Cancel）。
- 動き: 150–200ms ease-out。`prefers-reduced-motion: reduce` ならトランジションなし。
- フォーカスはパネル内に置く。閉じたらボードへ戻す。

実装は既存の `inspectorOpen && detail` を、グリッド第3列ではなく `position` オーバーレイにする。空の「Select a card」パネルは出さない（テストが DOM 上の文言を探すなら、非表示ノードとして残してよい）。

## 6. 残すもの / やらないもの

**残す**

- 4列と DnD（リフト、スナップバックなし）。
- ショートカット: `n` `/` `j` `k` `h` `l` `1–4` Escape、New task、ワードマーク、検索の `/` ヒント。
- 削除の二段確認。
- アクティビティは人間向けラベル（Moved など）。

**やらない**

- Issue リスト、ボード／リストのビュー切替。
- コマンドパレット。
- Trash の実画面（ボタンは今どおり死んでいる。別タスク）。
- URL にプロジェクトやカードを載せる。
- Native Glass のラベンダー面とガラスカード。

## 7. 触るファイル

- `packages/ui/src/theme.css` — トークン、フォント、`color-scheme`
- `packages/ui/src/*.module.css` — 面、境界、密度
- `packages/ui/src/TaskboardApp.tsx` / `.module.css` — オーバーレイ配置
- `packages/ui/src/Inspector.tsx` / `.module.css` — パネル見た目（確認フローは維持）
- `packages/ui/src/Card.module.css` — フラットカード
- `apps/web/index.html` — Inter を `<link>` で1本読む。`packages/ui` の `font-family` がそれを使う。新しいフォントパッケージは足さない。

CLI、Rust ドメイン、HTTP API は触らない。

## 8. 検証

- `pnpm --filter @taskboard/ui test` が緑。既存の「Select a card」アサーションを壊さない。
- 1440px: 未選択で4列が揃う。選択でボード幅が潰れない。
- Escape とパネル外クリックでオーバーレイが閉じる。
- 削除は確認なしに走らない。
- `prefers-reduced-motion` でオーバーレイが跳ねない。
- 実装後に `pnpm --filter web build`（`tb serve` は dist を見る）。

## 9. 成功条件

暗い Linear のボードに見える。カードを開いても4列が消えず、詳細は上に乗る。ショートカットと DnD は今と同じ。リスト画面は増えない。
