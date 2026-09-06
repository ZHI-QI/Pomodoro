export const UNITS = ['minute', 'hour', 'day'] as const;
export type Unit = (typeof UNITS)[number];

export const LIMITS: Record<Unit, [number, number]> = {
  minute: [1, 59],
  hour: [1, 23],
  day: [1, 30],
};

export const UNIT_LABEL: Record<Unit, string> = { minute: '分钟', hour: '小时', day: '天' };

export const PRESETS: { label: string; unit: Unit; amount: number }[] = [
  { label: '25分', unit: 'minute', amount: 25 },
  { label: '45分', unit: 'minute', amount: 45 },
  { label: '1时', unit: 'hour', amount: 1 },
  { label: '2时', unit: 'hour', amount: 2 },
  { label: '12时', unit: 'hour', amount: 12 },
  { label: '1天', unit: 'day', amount: 1 },
];

const SEC: Record<Unit, number> = { minute: 60, hour: 3600, day: 86400 };

export function clampAmount(unit: Unit, n: number): number {
  const [lo, hi] = LIMITS[unit];
  return Math.min(hi, Math.max(lo, Math.round(n)));
}

export function plannedSeconds(unit: Unit, amount: number): number {
  return amount * SEC[unit];
}
