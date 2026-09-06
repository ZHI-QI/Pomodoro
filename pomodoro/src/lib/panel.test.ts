import { describe, expect, it } from 'vitest';
import { clampAmount, outcomeBanner, plannedSeconds, PRESETS } from './panel';

describe('clampAmount', () => {
  it('minute 1-59', () => {
    expect(clampAmount('minute', 0)).toBe(1);
    expect(clampAmount('minute', 60)).toBe(59);
    expect(clampAmount('minute', 25)).toBe(25);
  });
  it('hour 1-23', () => {
    expect(clampAmount('hour', 0)).toBe(1);
    expect(clampAmount('hour', 24)).toBe(23);
  });
  it('day 1-30', () => {
    expect(clampAmount('day', 31)).toBe(30);
    expect(clampAmount('day', 1)).toBe(1);
  });
});

describe('plannedSeconds', () => {
  it('converts unit×amount', () => {
    expect(plannedSeconds('minute', 25)).toBe(1500);
    expect(plannedSeconds('hour', 2)).toBe(7200);
    expect(plannedSeconds('day', 1)).toBe(86400);
  });
});

describe('PRESETS', () => {
  it('contains spec presets', () => {
    expect(PRESETS.map((p) => p.label)).toEqual(['25分', '45分', '1时', '2时', '12时', '1天']);
  });
});

describe('outcomeBanner', () => {
  it('正常打开不显示横幅', () => {
    expect(outcomeBanner('opened')).toBeNull();
  });
  it('恢复与重置各自给出提示文案', () => {
    expect(outcomeBanner('recovered')).toContain('恢复');
    expect(outcomeBanner('reset')).toContain('重置');
  });
  it('未知状态按正常处理', () => {
    expect(outcomeBanner('something-else')).toBeNull();
  });
});
