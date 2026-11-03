const CONTROLS: Array<[string, string]> = [
  ['移動', '矢印キー / WASD'],
  ['B', 'Z — 斧を振る（ヤシの幹を3回で伐採）'],
  ['A', 'X (未使用)'],
  ['Y', 'C — ランタン所持の切替 (デバッグ)'],
  ['X', 'V (未使用)'],
  ['L / R', 'Q / E (未使用)'],
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
