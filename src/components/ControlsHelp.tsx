const CONTROLS: Array<[string, string]> = [
  ['移動', '矢印キー / WASD'],
  ['A', 'X — 調べる・拾う／決定'],
  ['B', 'Z — 手に持った道具を使う／戻る'],
  ['X', 'V — バッグ'],
  ['Y', 'C — 眠る／バッグ内で回す'],
  ['R', 'E — 走る'],
  ['START', 'Enter — 開始／ポーズ'],
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
