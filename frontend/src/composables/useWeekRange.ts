import { computed, type Ref, isRef } from 'vue';

const ISO_WEEK_RE = /^(\d{4})-W(\d{2})$/;

export function parseIsoWeek(s: string): Date | null {
  const m = ISO_WEEK_RE.exec(s);
  if (!m) return null;
  const year = Number(m[1]);
  const week = Number(m[2]);
  // ISO week 1 contains Jan 4. Compute Monday of that week, then add (week-1)*7 days.
  const jan4 = new Date(Date.UTC(year, 0, 4));
  const jan4Day = jan4.getUTCDay() || 7;
  const week1Monday = new Date(jan4);
  week1Monday.setUTCDate(jan4.getUTCDate() - (jan4Day - 1));
  const monday = new Date(week1Monday);
  monday.setUTCDate(week1Monday.getUTCDate() + (week - 1) * 7);
  // Validate the week actually exists in that year by round-tripping.
  if (formatIsoWeek(monday) !== s) return null;
  return monday;
}

export function formatIsoWeek(d: Date): string {
  const target = new Date(Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate()));
  const dayNum = target.getUTCDay() || 7;
  target.setUTCDate(target.getUTCDate() + 4 - dayNum);
  const yearStart = new Date(Date.UTC(target.getUTCFullYear(), 0, 1));
  const week = Math.ceil(((+target - +yearStart) / 86400000 + 1) / 7);
  return `${target.getUTCFullYear()}-W${String(week).padStart(2, '0')}`;
}

export function useWeekRange(input: string | Ref<string>) {
  const value = computed(() => (isRef(input) ? input.value : input));
  const monday = computed(
    () => parseIsoWeek(value.value) ?? parseIsoWeek(formatIsoWeek(new Date()))!,
  );
  const days = computed(() => {
    const out: Date[] = [];
    for (let i = 0; i < 7; i++) {
      const d = new Date(monday.value);
      d.setUTCDate(monday.value.getUTCDate() + i);
      out.push(d);
    }
    return out;
  });
  return { monday, days };
}
