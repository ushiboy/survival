# Castaway Island

無人島サバイバル（見下ろし型ARPG）。ゲームロジックと描画は Rust → WebAssembly、ページは Vite + React + TypeScript。

## 開発

前提: Rust (`wasm32-unknown-unknown` target), `wasm-pack`, Node, pnpm

```bash
pnpm install
pnpm dev      # wasm(debug) をビルドして Vite 起動
pnpm build    # wasm(release) + 型チェック + 本番ビルド
```

Rust 単体でフレームを PPM に書き出す確認用ツール（入力スクリプト = `ボタン(16進):フレーム数` の列）:

```bash
cd wasm && cargo run --release --example snapshot -- out.ppm "0200:90,0400:40"
```

主人公の全ポーズ×服装プリセットの一覧シート:

```bash
cd wasm && cargo run --release --example charsheet -- chars.ppm 4
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
  world/                地形・配置物・当たり判定・島の生成・採取(伐採)ルール (描画非依存)
  render/               ワールド → PPU入力 (地形タイル生成, HUD, カメラ, スプライト順, 時間帯ライティング)
  entity/               プレイヤー(移動・斧アクション)・カニ・カモメ・パーティクル・移動/衝突
  game/                 全体の進行 (step / render)
```

## SFC (Mode 1) 準拠の制約

- 256×224、15bit カラー、CGRAM 256色（BG 4bpp 16色×8 / OBJ 16色×8）。BG3(2bpp) は CGRAM 先頭32色を共有するため、地形は BG パレット 2〜7 を使用
- BG1: 地面、BG2: ヤシの葉など頭上オブジェ（優先度 High でスプライトより手前）、BG3: HUD
- 16×16 タイル単位でパレット・左右反転・優先度。スプライトは最大128枚、1ライン32枚/34スライバー
- カラー演算: 昼はスプライトパレット4〜7（影・波紋・水中の脚）をサブ画面と半透明合成。夜はサブ画面を減算して暗闇にする（HUD とパレット0〜3のスプライト＝主人公は対象外）
- 夜のランタン: BG3 マップの画面外に放射状の光マスク（2bpp・4段階＋ディザ）を置き、HDMA で HUD より下のラインだけ BG3 をメイン画面→サブ画面に切り替え、プレイヤー位置へスクロール。マスク外（ランタン未所持なら全体）は固定色＝完全な暗闇を減算。マスクの色を毎フレーム少し揺らして炎のちらつき
- 時間帯は CGRAM を毎フレーム再計算（パレット DMA 相当）、水面はパレットサイクル、波打ち際はタイル差し替えアニメ
- VRAM は「現在画面に必要なタイル量」をデバッグパネルに表示（64 KiB 以内）
