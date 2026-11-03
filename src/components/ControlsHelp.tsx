const CONTROLS: Array<[string, string]> = [
  ['移動', '矢印キー / WASD'],
  ['B', 'Z — 斧を振る（ヤシの幹を3回で伐採・角材が落ちる）／目覚めをスキップ／バッグ: 戻る・閉じる・持っている物を元に戻す'],
  ['A', 'X — 足元の物を拾う／目の前のヤシの木を揺する（長押しで揺すり続ける）／水汲み場で水を汲む／目覚めをスキップ／バッグ: 選ぶ・置く（重なった1つと入れ替え）'],
  ['X', 'V — バッグを開く・閉じる'],
  ['Y', 'C — バッグ: 持っている物を回す'],
  ['L', 'Q — ランタン所持の切替 (デバッグ)'],
  ['R', 'E — 服装の切替 (デバッグ)'],
  ['START', 'Enter — ポーズ'],
  ['SELECT', 'Shift — 時刻を3時間進める (デバッグ)'],
];

export function ControlsHelp() {
  return (
    <section className="panel">
      <h2>操作</h2>
      <dl className="kv">
        {CONTROLS.map(([k, v]) => (
          <div key={k}>
            <dt>{k}</dt>
            <dd>{v}</dd>
          </div>
        ))}
      </dl>
      <p className="note">ゲームパッドにも対応しています。</p>
    </section>
  );
}
