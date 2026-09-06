<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import { getStats, listRecent, type SessionDto, type StatsDto } from './ipc';

  let stats: StatsDto | null = null;
  let recent: SessionDto[] = [];
  let today = todayStr();

  $: maxFocus = Math.max(1, ...(stats?.byDay.map((d) => d.focusSec) ?? [1]));
  const WEEK = ['日', '一', '二', '三', '四', '五', '六'];
  const COLORS = ['#6366f1', '#22d3ee', '#8b5cf6', '#34d399', '#f472b6', '#fbbf24', '#f87171'];

  function dayLabel(day: string): string {
    const d = new Date(`${day}T12:00:00`);
    return WEEK[d.getDay()];
  }

  function hours(sec: number): string {
    return (sec / 3600).toFixed(1);
  }

  function hhmm(iso: string): string {
    return new Date(iso).toTimeString().slice(0, 5);
  }

  function dur(s: SessionDto): string {
    const sec = s.actualSec ?? s.plannedSec;
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    return h > 0 ? `${h}小时${m}分` : `${m}分钟`;
  }

  interface Seg { startPct: number; widthPct: number; color: string; name: string }

  // 时间轴：默认 09:00–21:00，自动扩展覆盖当天最早/最晚任务
  $: segments = buildSegments(recent.filter((s) => s.startedAt.slice(0, 10) === today));

  function todayStr(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  }

  function buildSegments(todays: SessionDto[]): Seg[] {
    if (todays.length === 0) return [];
    const mins = (iso: string) => {
      const d = new Date(iso);
      return d.getHours() * 60 + d.getMinutes();
    };
    let lo = 9 * 60;
    let hi = 21 * 60;
    for (const s of todays) {
      lo = Math.min(lo, mins(s.startedAt));
      const end = mins(s.endedAt ?? new Date().toISOString());
      hi = Math.max(hi, end);
    }
    const span = Math.max(1, hi - lo);
    return todays.map((s, i) => ({
      startPct: ((mins(s.startedAt) - lo) / span) * 100,
      widthPct: Math.max(1.5, (((mins(s.endedAt ?? new Date().toISOString())) - mins(s.startedAt)) / span) * 100),
      color: COLORS[i % COLORS.length],
      name: `${(s.note || '（无备注）').slice(0, 10)} ${dur(s)}`,
    }));
  }

  function statusLabel(s: SessionDto['status']): string {
    return { running: '运行', completed: '完成', aborted: '放弃' }[s];
  }

  async function load() {
    today = todayStr();
    stats = await getStats();
    recent = await listRecent(12);
  }

  onMount(() => {
    let stopped = false;
    let timer: ReturnType<typeof setInterval> | undefined;
    (async () => {
      await load();
      // 每 60s 轮询 + 窗口重新可见时刷新，修复跨午夜/长驻不更新
      timer = setInterval(() => {
        if (!stopped) load();
      }, 60_000);
    })();
    return () => {
      stopped = true;
      if (timer) clearInterval(timer);
    };
  });
</script>

<div class="stats">
  <h3><Icon name="chart" size={15} /> 专注统计</h3>

  {#if stats}
    <div class="kpis">
      <div class="kpi glass"><b>{hours(stats.todaySec)}</b><i>今日 · 小时</i></div>
      <div class="kpi glass"><b>{hours(stats.weekSec)}</b><i>本周 · 小时</i></div>
      <div class="kpi glass"><b>{stats.completedCount}</b><i>完成番茄 · 个</i></div>
    </div>

    <p class="lbl">最近 7 天 · 每日专注小时</p>
    <div class="bars glass">
      {#each stats.byDay as d}
        <div class="col">
          <div class="bar" style="height:{Math.round((d.focusSec / maxFocus) * 100)}%" />
          <em>{dayLabel(d.day)}</em>
        </div>
      {/each}
    </div>

    <p class="lbl">时间轴 · 今天</p>
    <div class="axis">
      {#each segments as seg}
        <div
          class="seg"
          style="left:{seg.startPct}%;width:{seg.widthPct}%;background:{seg.color}"
          title={seg.name}
        />
      {/each}
    </div>
    <div class="legend">
      {#each segments as seg}
        <span><i style="background:{seg.color}" />{seg.name}</span>
      {/each}
    </div>
  {/if}

  <p class="lbl">历史记录 · 最近</p>
  <table>
    {#each recent as s}
      <tr>
        <td class="nm">{s.note || '（无备注）'}</td>
        <td class="mono">{hhmm(s.startedAt)}–{s.endedAt ? hhmm(s.endedAt) : '…'}</td>
        <td class="mono">{dur(s)}</td>
        <td><span class="tag {s.status}">{statusLabel(s.status)}</span></td>
      </tr>
    {/each}
  </table>
</div>

<style>
  .stats { padding: 8px 6px; font-size: 12px; }
  h3 {
    margin: 4px 0 10px; font-size: 14px; color: var(--text);
    display: flex; align-items: center; gap: 6px; letter-spacing: 0.5px;
  }
  h3 :global(svg) { color: var(--accent2); }
  .kpis { display: flex; gap: 8px; }
  .kpi { flex: 1; padding: 10px; text-align: center; }
  .kpi b { display: block; font: 700 clamp(16px, 4.4vw, 19px)/1.2 Consolas, monospace; color: var(--accent2); }
  .kpi:nth-child(2) b { color: var(--accent3); }
  .kpi:nth-child(3) b { color: var(--ok); }
  .kpi i { font-style: normal; font-size: 10px; color: var(--muted); }
  .lbl { font-size: 10px; color: var(--muted); text-transform: uppercase; letter-spacing: 1.2px; margin: 12px 0 6px; font-weight: 600; }
  .bars { display: flex; align-items: flex-end; gap: 8px; height: 88px; padding: 10px; }
  .col { flex: 1; display: flex; flex-direction: column; justify-content: flex-end; align-items: center; height: 100%; gap: 4px; }
  .bar {
    width: 100%; min-height: 2px; border-radius: 4px 4px 0 0;
    background: linear-gradient(180deg, var(--accent2), var(--accent) 70%, var(--accent3));
    box-shadow: 0 0 10px rgba(99, 102, 241, 0.35);
    transition: height 0.5s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .col em { font-style: normal; font-size: 9px; color: var(--muted); }
  .axis {
    position: relative; height: 10px; border-radius: 5px;
    background: rgba(139, 150, 173, 0.15); overflow: hidden;
  }
  .seg { position: absolute; top: 0; height: 100%; border-radius: 3px; box-shadow: 0 0 6px rgba(99, 102, 241, 0.3); }
  .legend { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 6px; font-size: 10px; color: var(--muted); }
  .legend i { display: inline-block; width: 8px; height: 8px; border-radius: 2px; margin-right: 4px; }
  table { width: 100%; border-collapse: collapse; }
  td { padding: 6px 4px; border-bottom: 1px dashed var(--glass-border); color: var(--text); }
  td.nm { max-width: 110px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  td.mono { font: 10px Consolas, monospace; color: var(--muted); }
  .tag { font-size: 9px; padding: 2px 8px; border-radius: 999px; }
  .tag.completed { background: rgba(52, 211, 153, 0.15); color: var(--ok); }
  .tag.aborted { background: rgba(248, 113, 113, 0.15); color: var(--danger); }
  .tag.running { background: rgba(99, 102, 241, 0.2); color: var(--accent3); }
</style>
