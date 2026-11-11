# Castaway Island

無人島サバイバル（見下ろし型ARPG）。ゲームロジックと描画は Rust → WebAssembly、ページは Vite + React + TypeScript。

## 仕様

ゲームのコンセプトと現在の仕様は [docs/](docs/README.md) にまとめています。素材・道具・食べ物は [docs/items/](docs/items/README.md)、レシピは [docs/recipes/](docs/recipes/README.md) に1つずつファイルを分けています。

## 開発

前提: Rust (`wasm32-unknown-unknown` target), `wasm-pack`, Node, pnpm

```bash
pnpm install
pnpm dev      # wasm(debug) をビルドして Vite 起動
pnpm build    # wasm(release) + 型チェック + 本番ビルド
pnpm test     # Rust のテスト（cargo test --manifest-path wasm/Cargo.toml）
```

テストは Rust 側にだけあり、ネイティブで動きます（ブラウザは要りません）。単体テストは各ファイルの末尾の `#[cfg(test)] mod tests`、`Game` を外から動かすシナリオテストは `wasm/tests/` にあります。セーブの変換のテストは、古い版のセーブの見本（`wasm/src/game/save_samples/`）を読みます。

URL に `?debug=true` を付けると（例: `http://localhost:5173/?debug=true`）デバッグパネルが出て、好きなアイテムを個数を指定してバッグに入れたり、全レシピを解放したりできます。

Rust 単体でフレームを PPM に書き出す確認用ツール（入力スクリプト = `ボタン(16進):フレーム数` の列）:

```bash
cd wasm && cargo run --release --example snapshot -- out.ppm "0200:90,0400:40"
```

終了時のプレイヤーのタイル座標を標準エラーに出力します。環境変数 `SNAP_AT=x,y` を付けると、目覚めを飛ばしてそのセルから始めます。 `SNAP_GIVE=stone,vine,branch` のように指定すると、それらのアイテムをバッグに入れた状態で始めます。 `SNAP_CRAB=24` のように指定すると、プレイヤーの右にその距離だけ離して蟹を置きます。 `SNAP_HP=16` のように指定すると、体力がその値（最大64）の状態で始めます。 `SNAP_FOOD=0`・`SNAP_WATER=0` も同じように、満腹度・水分をその値にして始めます。 `SNAP_HOURS=12` のように指定すると、その時間だけ時刻を進めてから（`SNAP_GIVE` より前に）始めます。 `SNAP_FISH=10,40` のように指定すると、プレイヤーの足元からその距離だけ離して魚を置きます。 `SNAP_RECIPES=all` を付けると、全レシピを思いついた状態で始めます（付けないと、持っている材料から思いついたレシピしか CRAFT に出ません）。

タイトル画面は飛ばして始まります（`SNAP_TITLE=1` を付けるとタイトル画面から始まります）。セーブまわりは `SNAP_RELOAD=1`（最後にセーブして、そのセーブから再開したゲームを描く。再開したゲームが同じ中身をセーブし直すかも調べる）、`SNAP_SAVE_OUT=path`（セーブを書き出す）、`SNAP_SAVE_IN=path`（そのセーブがある状態で始める）で確かめられます。実行中のセーブ・削除は「saved game: …」と出力します。`SNAP_EVERY_FRAME=1` を付けると、ブラウザと同じく毎フレーム描画します（地面のタイルのストリーミングも毎フレーム動く）。終了時に、地面用に持っているタイルの数も出力します。

地面のタイルのストリーミングの確認（島を5×5に並べた大きなマップで画面を端から端まで動かし、画面内のタイルが毎回作り直したものと1ピクセルずつ一致するかを調べる）:

```bash
cd wasm && cargo run --release --example stream_check
```

島の地図（設計図や島の作り方を変えたときの確認用。使い方は [docs/island.md](docs/island.md#確認ツール)）。`wasm/target/map/` に、設計の地図 `plan.png`、ゲームの見た目の全景 `view.png`、特別な場所の画面の一覧 `spots.png`、前と比べて変わった所 `plan-diff.png`・`view-diff.png` を書き出します。漂着地点から行ける範囲も出力します:

```bash
pnpm map                          # すべて描く（数秒）
pnpm map --quick                  # 設計の地図だけ
pnpm map --area 55,8,72,22        # 全景をこの範囲のマス（x0,y0,x1,y1）だけ
pnpm map --shrink 2               # 全景を縮めて（2×2 ピクセルを1つに）
pnpm map --set-base               # 今の状態を比べる元にする（--clear-base で直前の実行と比べるのに戻す）
pnpm map:watch                    # wasm/src が変わるたびに描き直す（オプションも渡せる）
```

ツールは `tools` プロファイル（release から LTO を抜いたもの）でビルドするので、描き変えてから数秒で描き直せます。

島に置く物とアイテムの絵の一覧シート（絵を足したり直したりしたときの確認用）。`wasm/target/art/sheet.png` に、名前を付けて並べます。島に置く物（ヤシ・広葉樹・松などの木はまるごと、切り株、茂み、つるとその支柱、岩、流木、貝、花、松ぼっくり）は、それぞれの地面（砂か草）の上に周り1マスを付けて描きます。アイテムは `ItemKind::ALL` のすべてを、バッグのアイコン（向きを変えられるものは横向きも）、砂の上・草の上に落ちた絵の順に並べます。どれもゲームの地形タイルと PPU で昼の明るさで描きます。前の実行（または比べる元）から変わったカードは `sheet-diff.png` に前後を並べて出します:

```bash
pnpm art                          # すべて描く（既定は 3 倍）
pnpm art palm "pine cone"         # 名前にどれかを含むカードだけ（大文字小文字は問わない）
pnpm art:watch -- "pine stump" --scale 6  # 新しい絵を試すとき: その1枚だけを大きく、直すたびに描き直す
pnpm art --scale 4                # 4 倍で描く
pnpm art --set-base               # 今の状態を比べる元にする（--clear-base で直前の実行と比べるのに戻す）
pnpm art:watch                    # wasm/src が変わるたびに描き直す（オプションや名前も渡せる）
```

ゲームに島の配置物（`Feature`）を足すと、`wasm/examples/art.rs` の `on_sheet` がコンパイルできなくなるので、そこと `specimens` に足してください。

主人公の全ポーズ×服装プリセットの一覧シート:

```bash
cd wasm && cargo run --release --example charsheet -- chars.ppm 4
```
水汲み場のセルと、スポーン地点に近いヤシの木（幹の根元のセル）・残りの椰子の実の数・茂み・ツル植物・大岩・流木・貝殻・広葉樹のセル:

```bash
cd wasm && cargo run --release --example palms -- 1
```

## 構成

```
src/                    React / TS (表示と入力のみ)
  game/wasmCore.ts      wasm のラッパー (フレームはコピーせず wasm メモリを直接参照)
  game/input.ts         キーボード/ゲームパッド → SFCパッドのボタンword
  game/gameLoop.ts      60.1Hz 固定ステップのループ
  components/           画面・操作説明・デバッグパネル
wasm/src/               Rust コア
  lib.rs                JS向けファサード (wasm_bindgen はここだけ)
  common/               固定小数点・乱数・入力
  ppu/                  SFC(Mode 1)風 PPU: CGRAM, BG1/BG2/BG3, OBJ, カラー演算, ウィンドウ, HDMAテーブル
  assets/               パレット割り当て, 手描きスプライト(テキスト), 手続き生成の地形/オブジェ/HUD
  assets/character/     主人公: ポーズ骨格 + 装備レイヤー(頭・上衣・下衣・足)からスプライトを生成
  world/                地形・配置物・当たり判定・島の生成・採取(伐採)ルール・区画 (描画非依存)
  render/               ワールド → PPU入力 (地形タイル生成, HUD, カメラ, スプライト順, 時間帯ライティング)
  entity/               プレイヤー(移動・斧アクション)・カニ・カモメ・パーティクル・移動/衝突
  game/                 全体の進行 (step / render)
wasm/examples/          開発用ツール (スナップショット, 島の地図, 絵の一覧, ストリーミングの確認など)
scripts/                開発用スクリプト (島の地図・絵の一覧を描き直す見張り)
```

## SFC (Mode 1) 準拠の制約

- 256×224、15bit カラー、CGRAM 256色（BG 4bpp 16色×8 / OBJ 16色×8）。BG3(2bpp) は CGRAM 先頭32色を共有するため、地形は BG パレット 2〜7 を使用
- BG1: 地面、BG2: ヤシの葉など頭上オブジェ（優先度 High でスプライトより手前）、BG3: HUD
- 16×16 タイル単位でパレット・左右反転・優先度。スプライトは最大128枚、1ライン32枚/34スライバー
- カラー演算: 昼はスプライトパレット4〜7（影・波紋・水中の脚）をサブ画面と半透明合成。夜はサブ画面を減算して暗闇にする（HUD とパレット0〜3のスプライト＝主人公は対象外）
- 夜の明かり（持ち歩いている松明と、すべての焚き火）: 放射状の光マスク（2bpp・4段階＋ディザ、16×16タイル）を、BG3 マップの画面外にある画面サイズの「光の地図」へ光源ごとに毎フレーム並べる（ワールドの 8px グリッドに揃える）。光が重なるセルは明るい方を取ったタイルを予備タイル（最大96枚）へ書き込む（ブランク中の VRAM 転送相当）。HDMA で HUD より下のラインの BG3 をメイン画面→サブ画面に切り替え、カメラの端数だけスクロールして光の地図を重ねる。光の外は固定色＝完全な暗闇を減算。マスクの色を毎フレーム少し揺らして炎のちらつき
- 時間帯は CGRAM を毎フレーム再計算（パレット DMA 相当）、水面はパレットサイクル、波打ち際はタイル差し替えアニメ
- VRAM は「現在画面に必要なタイル量」をデバッグパネルに表示（64 KiB 以内）
- 地面のタイルは 32×32 マスの区画単位でストリーミングする（`render/tileset.rs`）。画面付近の区画だけを作り、離れた区画のタイルは捨てる。タイルは使っているセルの数を数え、どのセルも使わなくなったものは VRAM から解放してスロットを再利用する（スクロールに合わせた CHR 転送相当）
- 蟹・魚・落ちたアイテムは、画面付近の起きている区画でだけ動かす（`game/regions.rs`）。眠っていた区画が起きるとき、眠っていた時間を受け取る
