import { describe, expect, it } from 'vitest';
import { HARD_LIMIT, noteLen } from './noteCounter';

describe('noteLen', () => {
  it('counts code points, 中文=1', () => {
    expect(noteLen('')).toBe(0);
    expect(noteLen('abc')).toBe(3);
    expect(noteLen('番茄钟')).toBe(3);
    expect(noteLen('😀')).toBe(1);
  });
  it('hard limit is 5000', () => {
    expect(HARD_LIMIT).toBe(5000);
  });
});
