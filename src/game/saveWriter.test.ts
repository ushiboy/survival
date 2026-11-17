import { describe, expect, it, vi } from 'vitest';
import { createSaveWriter } from './saveWriter';

/** A writer whose idle callbacks run only when `idle()` is called. */
function setup() {
  const written: string[] = [];
  let queued: (() => void)[] = [];
  const writer = createSaveWriter({
    write: (op) => written.push(op),
    whenIdle: (cb) => queued.push(cb),
  });
  const idle = () => {
    const run = queued;
    queued = [];
    run.forEach((cb) => cb());
  };
  return { writer, written, idle, queued: () => queued.length };
}

describe('セーブの書き込み', () => {
  it('セーブはすぐには書かず、空いた時間に書く', () => {
    const { writer, written, idle } = setup();
    writer.offer('{"a":1}');
    expect(written).toEqual([]);
    idle();
    expect(written).toEqual(['{"a":1}']);
  });

  it('書く前に次のセーブが来たら、最新の1件だけを書く', () => {
    const { writer, written, idle, queued } = setup();
    writer.offer('{"a":1}');
    writer.offer('{"a":2}');
    expect(queued()).toBe(1);
    idle();
    expect(written).toEqual(['{"a":2}']);
  });

  it('セーブを消すときはすぐ消し、書くのを待っていたセーブは捨てる', () => {
    const { writer, written, idle } = setup();
    writer.offer('{"a":1}');
    writer.offer('');
    expect(written).toEqual(['']);
    idle();
    expect(written).toEqual(['']);
  });

  it('flush で待っているセーブをその場で書く', () => {
    const { writer, written, idle } = setup();
    writer.offer('{"a":1}');
    writer.flush();
    expect(written).toEqual(['{"a":1}']);
    idle();
    expect(written).toEqual(['{"a":1}']);
  });

  it('待っているセーブがなければ flush は何も書かない', () => {
    const write = vi.fn();
    const writer = createSaveWriter({ write, whenIdle: () => {} });
    writer.flush();
    expect(write).not.toHaveBeenCalled();
  });
});
