# Castaway Island

無人島サバイバルの見下ろし型アクション RPG です。ゲームロジックと描画は Rust → WebAssembly（スーパーファミコン Mode 1 風のソフトウェア PPU）、ページは Vite + React + TypeScript で作っています。

ゲームのコンセプト・遊び方・仕様は [docs/](docs/README.md) にまとめています。このファイルには、開発に必要なことだけを書きます。

## 必要なもの

- [mise](https://mise.jdx.dev/) と [rustup](https://rustup.rs/)

バージョンは固定しています。Node・pnpm・wasm-pack は `mise.toml`、Rust（`wasm32-unknown-unknown` target と clippy・rustfmt を含む）は `rust-toolchain.toml` にあります。

```bash
mise install         # Node・pnpm・wasm-pack を入れる（Rust はこのディレクトリで cargo を初めて動かしたときに rustup が入れる）
```

バージョンを上げるときは、これらのファイルを書き換えます。

## コマンド

```bash
pnpm install
pnpm dev             # wasm(debug) をビルドして Vite 起動
pnpm build           # wasm(release) + 型チェック + 本番ビルド
pnpm preview         # 本番ビルドを開く
pnpm test            # Rust のテスト（cargo test --manifest-path wasm/Cargo.toml）
pnpm test:web        # TypeScript のテスト（Vitest + Testing Library、jsdom）。見張って回すなら pnpm test:web:watch
pnpm exec tsc -b     # 型チェック
pnpm fmt             # Rust のコードを整形（rustfmt。設定は wasm/rustfmt.toml）。確かめるだけなら pnpm fmt:check
```

## テスト

- Rust のテストはネイティブで動きます（ブラウザは要りません）。単体テストは各ファイルの末尾の `#[cfg(test)] mod tests`、`Game` を外から動かすシナリオテストは `wasm/tests/` にあります。セーブのテストは、版ごとのセーブの見本（`wasm/src/game/save_samples/`）を読みます（[セーブと再開](docs/save.md#形式を変えるとき)）。
- TypeScript のテストは、対象のファイルの隣に `*.test.ts(x)` として置きます（例: `src/game/touch.test.ts`）。wasm はビルドせずに動きます。wasm を使う部品（`GameScreen`）のテストは、`vi.mock` で `game/wasmCore` とゲームのループを差し替えます。jsdom にない要素の位置・当たり判定（`elementFromPoint`）・`matchMedia` は、`src/test/dom.ts` の補助で用意します。

## ブラウザで確かめる

- **デバッグ**: URL に `?debug=true` を付けると（例: `http://localhost:5173/?debug=true`）「デバッグ」ボタンが出ます。できること（アイテムを持たせる、全レシピの解放、決まった場所へ移す、時刻を進めるなど）は [docs の「デバッグ機能」](docs/README.md#デバッグ機能)にあります。
- **タッチ操作の画面**: `pointer: coarse` の端末ではタッチ操作の画面になります（[操作の手段](docs/README.md#操作の手段)）。PC でこの画面にする URL のオプション（`?touch=true`）も「デバッグ機能」にあります。同じ LAN のスマートフォンから開くには `pnpm dev --host` で起動します。
- **PWA**: 開発サーバー（`pnpm dev`）では Service Worker を登録しないので、インストールやオフラインを確かめるときは `pnpm build` のあと `pnpm preview` で開きます。マニフェストは `public/manifest.webmanifest`、Service Worker は `scripts/service-worker.js` が元で、`vite build` のときに `scripts/pwa.ts`（Vite のプラグイン）がページとビルドの全ファイルの一覧と版（それらの中身のハッシュ）を書き込んで `dist/sw.js` にします。

## 開発用ツール

ツールは `wasm/examples/` にあり、`tools` プロファイル（release から LTO を抜いたもの）か release でビルドします。

### スナップショット

Rust だけで任意のフレームを PPM に書き出します（入力スクリプト = `ボタン(16進):フレーム数` の列）。

```bash
cd wasm && cargo run --release --example snapshot -- out.ppm "0200:90,0400:40"
```

終了時のプレイヤーのタイル座標と、地面用に持っているタイルの数を標準エラーに出力します。タイトル画面は飛ばして始まります。環境変数で始める状態を変えられます。

| 環境変数 | 始める状態 |
|---|---|
| `SNAP_AT=x,y` | 目覚めを飛ばしてそのセルから |
| `SNAP_GIVE=stone,vine,branch` | それらのアイテムをバッグに入れて |
| `SNAP_RECIPES=all` | 全レシピを思いついた状態で（付けないと、持っている材料から思いついたレシピしか CRAFT に出ない） |
| `SNAP_HOURS=12` | その時間だけ時刻を進めて（`SNAP_GIVE` より前） |
| `SNAP_HP=16` | 体力をその値（最大64）にして |
| `SNAP_FOOD=0`・`SNAP_WATER=0` | 満腹度・水分をその値にして |
| `SNAP_UPSET=1` | お腹を壊した状態で |
| `SNAP_CRAB=24` | プレイヤーの右にその距離だけ離して蟹を置いて |
| `SNAP_FISH=10,40` | プレイヤーの足元からその距離だけ離して魚を置いて |
| `SNAP_TITLE=1` | タイトル画面から |
| `SNAP_EVERY_FRAME=1` | ブラウザと同じく毎フレーム描画する（地面のタイルのストリーミングも毎フレーム動く） |

セーブまわりの変数（`SNAP_RELOAD`・`SNAP_SAVE_OUT`・`SNAP_SAVE_IN`）は [セーブと再開の「確認ツール」](docs/save.md#確認ツール)にあります。

### 島の地図

設計図や島の作り方を変えたときに使います。`wasm/target/map/` に設計の地図・全景・特別な場所の一覧・前との差分を書き出し、漂着地点から行ける範囲と島にある物の数を出力します。オプションと出力の見方は [島の設計の「確認ツール」](docs/island.md#確認ツール)にあります。

```bash
pnpm map             # すべて描く（数秒）
pnpm map:watch       # wasm/src が変わるたびに描き直す（オプションも渡せる）
```

### 絵の一覧

島に置く物とアイテムの絵を足したり直したりしたときに使います。`wasm/target/art/sheet.png` に名前を付けて並べます。島に置く物は、それぞれの地面（砂か草）の上に周り1マスを付けて描きます。アイテムは `ItemKind::ALL` のすべてを、バッグのアイコン（向きを変えられるものは横向きも）、砂の上・草の上に落ちた絵の順に並べます。どれもゲームの地形タイルと PPU で昼の明るさで描きます。前の実行（または比べる元）から変わったカードは `sheet-diff.png` に前後を並べて出します。

```bash
pnpm art                          # すべて描く（既定は 3 倍）
pnpm art palm "pine cone"         # 名前にどれかを含むカードだけ（大文字小文字は問わない）
pnpm art:watch -- "pine stump" --scale 6  # 新しい絵を試すとき: その1枚だけを大きく、直すたびに描き直す
pnpm art --scale 4                # 4 倍で描く
pnpm art --set-base               # 今の状態を比べる元にする（--clear-base で直前の実行と比べるのに戻す）
pnpm art:watch                    # wasm/src が変わるたびに描き直す（オプションや名前も渡せる）
```

島の配置物（`Feature`）を足すと `wasm/examples/art.rs` の `on_sheet` がコンパイルできなくなるので、そこと `specimens` に足してください。

### そのほか

```bash
cd wasm && cargo run --release --example stream_check      # 地面のタイルのストリーミングの確認
cd wasm && cargo run --release --example charsheet -- chars.ppm 4  # 主人公の全ポーズ×服装プリセットの一覧
cd wasm && cargo run --release --example palms -- 1        # 水汲み場・漂着地点の近くの資源のセルの一覧
```

- `stream_check`: 島を5×5に並べた大きなマップで画面を端から端まで動かし、画面内のタイルが毎回作り直したものと1ピクセルずつ一致するかを調べます（[描画](docs/rendering.md#vram-とストリーミング)）。
- `palms`: 水汲み場のセル（すべて）と、漂着地点に近いヤシの木（幹の根元のセル）・残りの椰子の実の数・茂み・ツル植物・大岩・流木・貝殻・広葉樹のセルを出力します。

## 構成

```
src/                    React / TS (表示と入力のみ)
  game/wasmCore.ts      wasm のラッパー (フレームはコピーせず wasm メモリを直接参照)
  game/input.ts         キーボード/ゲームパッド → SFCパッドのボタンword
  game/touch.ts         画面上の仮想コントローラのマルチタッチ → ボタンword
  game/touchMode.ts     タッチ用レイアウトにするかの判定 (pointer: coarse / ?touch=)
  game/gameLoop.ts      60.1Hz 固定ステップのループ
  game/serviceWorker.ts Service Worker の登録（本番ビルドのときだけ）
  components/           画面・仮想コントローラ・操作説明・表示・デバッグのダイアログ
  i18n/                 ページの文言の日本語・英語（use-intl）。messages/ja.ts が元で、en.ts は同じ項目をそろえる。navigator.language で切り替え
public/                 そのまま配る物: favicon（favicon.svg が元の 16×16 ドット絵。favicon.ico、
                        ホーム画面用の apple-touch-icon.png、PWA 用の icon-192/512.png は同じ絵を整数倍に
                        拡大したもの。icon-maskable-512.png は丸く切り抜かれてもよいよう周りに海を足したもの）
                        と PWA のマニフェスト（manifest.webmanifest）
wasm/src/               Rust コア
  lib.rs                JS向けファサード (wasm_bindgen はここだけ)
  common/               固定小数点・乱数・入力
  ppu/                  SFC(Mode 1)風 PPU: CGRAM, BG1/BG2/BG3, OBJ, カラー演算, ウィンドウ, HDMAテーブル
  assets/               パレット割り当て, 手描きスプライト(テキスト), 手続き生成の地形/オブジェ/HUD
  assets/character/     主人公: ポーズ骨格 + 装備レイヤー(頭・上衣・下衣・足)からスプライトを生成
  world/                地形・配置物・当たり判定・島の生成・採取ルール・区画 (描画非依存)
  render/               ワールド → PPU入力 (地形タイル生成, HUD, カメラ, スプライト順, 時間帯ライティング)
  entity/               プレイヤー(移動・行動)・カニ・魚・カモメ・焚き火・落ちたアイテム・パーティクル・移動/衝突
  game/                 全体の進行 (step / render)・バッグ・メニュー・釣り・区画の眠り・セーブ
  items.rs, crafting.rs アイテムとレシピ
wasm/tests/             Game を外から動かすシナリオテスト
wasm/examples/          開発用ツール (スナップショット, 島の地図, 絵の一覧, ストリーミングの確認など)
scripts/                開発用スクリプト (島の地図・絵の一覧を描き直す見張り) と、ビルドで dist/sw.js を作る
                        Service Worker の元（service-worker.js）と Vite のプラグイン（pwa.ts）
docs/                   ゲームの仕様（コンセプト・ルール・アイテム・レシピ・島・セーブ・描画）
```

SFC (Mode 1) の制約と描画の仕組みは [docs/rendering.md](docs/rendering.md) にあります。

## 開発の決まり

コードと仕様（docs）は同じ変更の中で更新します。テストの書き方・docs の更新先・コミットの決まりなどは [AGENTS.md](AGENTS.md) にあります。

## ライセンス

[MIT](LICENSE)
