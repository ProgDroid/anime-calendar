import { describe, it, expect } from 'vitest';
import { useWeekRange, parseIsoWeek, formatIsoWeek } from '../useWeekRange';

describe('parseIsoWeek', () => {
  it('parses YYYY-Www to a Monday Date', () => {
    const d = parseIsoWeek('2026-W18');
    expect(d).not.toBeNull();
    expect(d!.getUTCDay()).toBe(1); // Monday
    expect(formatIsoWeek(d!)).toBe('2026-W18');
  });
  it('returns null for invalid input', () => {
    expect(parseIsoWeek('garbage')).toBeNull();
    expect(parseIsoWeek('2026-W')).toBeNull();
    expect(parseIsoWeek('2025-W53')).toBeNull(); // 2025 has only 52 ISO weeks
  });
});

describe('formatIsoWeek', () => {
  it('round-trips with parseIsoWeek for several weeks', () => {
    for (const w of ['2026-W01', '2026-W18', '2026-W52']) {
      const d = parseIsoWeek(w);
      expect(d).not.toBeNull();
      expect(formatIsoWeek(d!)).toBe(w);
    }
  });
});

describe('useWeekRange', () => {
  it('produces 7 consecutive days starting Monday', () => {
    const { days } = useWeekRange('2026-W18');
    expect(days.value).toHaveLength(7);
    expect(days.value[0]!.getUTCDay()).toBe(1); // Mon
    expect(days.value[3]!.getUTCDay()).toBe(4); // Thu
    expect(days.value[6]!.getUTCDay()).toBe(0); // Sun
    // Each day is exactly 24h after the previous
    for (let i = 1; i < 7; i++) {
      const delta = days.value[i]!.getTime() - days.value[i - 1]!.getTime();
      expect(delta).toBe(86400000);
    }
  });

  it('handles year-boundary week 2026-W01', () => {
    const { days } = useWeekRange('2026-W01');
    expect(days.value).toHaveLength(7);
    expect(days.value[0]!.getUTCDay()).toBe(1);
  });

  it('falls back to current week for invalid input', () => {
    const { days } = useWeekRange('not-a-week');
    expect(days.value).toHaveLength(7);
    expect(days.value[0]!.getUTCDay()).toBe(1);
  });
});
