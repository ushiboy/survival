# Castaway Island

無人島サバイバル（見下ろし型ARPG）。ゲームロジックと描画は Rust → WebAssembly、ページは Vite + React + TypeScript。

## 開発

前提: Rust (`wasm32-unknown-unknown` target), `wasm-pack`, Node, pnpm

```bash
pnpm install
pnpm dev      # wasm(debug) をビルドして Vite 起動
pnpm build    # wasm(release) + 型チェック + 本番ビルド
```

Rust 単体でフレームを PPM に書き出す確認用ツール:

```bash
cd wasm && cargo run --example snapshot -- out.ppm 60 0
```

## 構成

```
src/                    React / TS (表示と入力のみ)
  game/wasmCore.ts      wasm のラッパー (フレームはコピーせず wasm メモリを直接参照)
  game/input.ts         キーボード/ゲームパッド → NESパッドのボタンbyte
  game/gameLoop.ts      60.1Hz 固定ステップのループ
  components/           画面・操作説明・デバッグパネル
wasm/src/               Rust コア
  lib.rs                JS向けファサード (wasm_bindgen はここだけ)
  common/               固定小数点・乱数・入力
  ppu/                  NES風 PPU: パレットRAM, CHRバンク, OAM, ライン単位描画
  assets/               ドット絵 (テキスト形式) とパレット割り当て
  world/                地形・配置物・当たり判定・島の生成 (描画非依存)
  render/               ワールド → PPU入力 (オートタイル, HUD, カメラ, メタスプライト)
  entity/               プレイヤー・カニ・移動/衝突
  game/                 全体の進行 (step / render)
```

## NES 準拠の制約

- 256×240、NES 64色マスターパレット、BG 4パレット + スプライト 4パレット（各3色 + 共通色/透明）
- BG は 16×16 メタタイル単位で1パレット（属性テーブル相当）、アニメはCHRバンク切替相当
- スプライトは 8×8 を最大64枚、1ラインあたり8枚（超過時はフリッカー）、左右反転でCHR節約
- 夜はパレット操作（輝度-1 + 青寄せ）で表現
