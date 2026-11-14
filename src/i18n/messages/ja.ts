/**
 * The page's text in Japanese, the reference: `en.ts` must have the same keys
 * (the type checker holds it to that).
 */
const ja = {
  controls: {
    title: '操作',
    dpad: '十字キー',
    arrowKeys: '矢印キー / WASD',
    roles: {
      move: '移動',
      act: '調べる・拾う／決定',
      useTool: '手に持った道具を使う／戻る',
      bag: 'バッグ',
      rotate: 'バッグ内で回す',
      sleep: '眠る',
      run: '走る',
      start: '開始／ポーズ',
    },
    note: 'ゲームパッドにも対応しています。スマートフォンでは画面のボタンで操作できます。',
  },
  touchHelp: {
    open: '操作説明',
    runNote: 'R は押しながら十字キーで走ります。',
    skip: '次から表示しない',
    close: '閉じる',
  },
  display: {
    title: '表示',
    scale: '倍率',
    crtAspect: 'CRTアスペクト比 (8:7)',
    scanlines: '走査線',
    spriteLimit: 'スプライト制限 (32個・34枚/ライン)',
    vram: 'VRAM {used} / 64 KiB (画面内)',
  },
  debug: {
    title: 'デバッグ',
    item: 'アイテム',
    count: '個数',
    give: '持たせる',
    giveStack: '1スタック (×{max})',
    given: '{item} ×{given} を持たせました',
    givenSome: '{item} ×{given} を持たせました（{left} 個は入りきりません）',
    noRoom: 'バッグに {item} が入る場所がありません',
    learnRecipes: '全レシピを解放',
    learnedRecipes: '全レシピを解放しました',
    move: '移動',
    choosePlace: '場所を選ぶ',
    cell: 'マス',
    moveHere: '移す',
    moved: '{place} の ({x}, {y}) へ移しました',
    nowhereToStand: '({x}, {y}) の近くに立てる所がありません',
    vitals: '状態',
    vitalsPresets: {
      full: { label: '全回復', done: '体力・満腹度・水分を満タンにしました' },
      hungry: { label: '飢え', done: '満腹度を0にしました' },
      thirsty: { label: '渇き', done: '水分を0にしました' },
      dying: { label: '瀕死', done: '体力を1にしました' },
    },
    condition: '体調',
    upsetStomach: { label: 'お腹を壊す', done: 'お腹を壊した状態にしました' },
    cure: { label: '治す', done: '体調を元に戻しました' },
    keys: 'Shift (SELECT) — 時刻を3時間進める／O — 服装の切替',
  },
};

export default ja;
