<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import Stats from './Stats.svelte';
  import { clampAmount, LIMITS, plannedSeconds, PRESETS, UNIT_LABEL, UNITS, type Unit } from './panel';
  import { GOAL, HARD_LIMIT, noteLen } from './noteCounter';
  import {
    abortSession,
    getActiveSession,
    getSettings,
    listToday,
    saveSettings,
    startSession,
    type SessionDto,
    type SettingsDto,
  } from './ipc';
  import { initTheme } from './theme';

  let unit: Unit = 'minute';
  let amount = 25;
  let custom = false;
  let note = '';
  let active: SessionDto | null = null;
  let activeRemaining = 0;
  let today: SessionDto[] = [];
  let settings: SettingsDto | null = null;
  let statsOpen = false;
  let settingsOpen = false;

  $: chars = noteLen(note);
  $: goalMet = chars >= GOAL;
  $: overHard = chars > HARD_LIMIT;

  async function refresh() {
    active = await getActiveSession();
    if (active) activeRemaining = active.plannedSec;
    today = await listToday();
  }

  function pickPreset(p: (typeof PRESETS)[number]) {
    unit = p.unit;
    amount = p.amount;
    custom = false;
  }

  function setUnit(u: Unit) {
    unit = u;
    amount = clampAmount(u, amount);
  }

  function changeAmount(delta: number) {
    amount = clampAmount(unit, amount + delta);
  }

  async function start() {
    active = await startSession(note.trim().slice(0, HARD_LIMIT), unit, amount);
    activeRemaining = active.plannedSec;
    note = '';
    today = await listToday();
  }

  async function giveUp() {
    if (!active) return;
    if (!confirm('确定放弃当前番茄？')) return;
    active = await abortSession(active.id);
    await refresh();
  }

  async function toggleSound() {
    if (!settings) return;
    settings = await saveSettings({ ...settings, sound: !settings.sound });
  }
  async function toggleNotify() {
    if (!settings) return;
    settings = await saveSettings({ ...settings, notification: !settings.notification });
  }
  async function setAutostart() {
    if (!settings) return;
    settings = await saveSettings({ ...settings, autostart: !settings.autostart });
  }
  async function setTheme(e: Event) {
    if (!settings) return;
    const theme = (e.target as HTMLSelectElement).value as SettingsDto['theme'];
    settings = await saveSettings({ ...settings, theme });
  }

  function barWidth(s: SessionDto): string {
    const px = Math.min(120, Math.max(24, s.plannedSec / 60));
    return `${px}px`;
  }

  function timeStr(sec: number): string {
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    return h > 0 ? `${h}小时${m}分` : `${m}分钟`;
  }

  onMount(() => {
    let off: (() => void) | undefined;
    (async () => {
      settings = await getSettings();
      await initTheme(settings.theme);
      await refresh();
      off = await listen<{ remainingSec: number; sessionId: number }>('tick', (e) => {
        if (active && e.payload.sessionId === active.id) activeRemaining = e.payload.remainingSec;
      });
      await listen('session_done', refresh);
    })();
    return () => off?.();
  });
</script>

<svelte:window on:keydown={(e) => e.key === 'Escape' && (statsOpen = false)} />

<main>
  <header>
    <b>番茄钟</b>
    <span class="spacer" />
    <button class="icon" title="统计" on:click={() => (statsOpen = true)}>📊</button>
    <button class="icon" title="设置" on:click={() => (settingsOpen = !settingsOpen)}>⚙</button>
  </header>

  {#if active}
    <section class="running">
      <div>
        <b>{active.note || '（无备注）'}</b>
        <i>剩余 {timeStr(activeRemaining)}</i>
      </div>
      <button class="danger" on:click={giveUp}>放弃</button>
    </section>
  {/if}

  <section>
    <p class="lbl">快速时长</p>
    <div class="chips">
      {#each PRESETS as p}
        <button class="chip" class:on={!custom && unit === p.unit && amount === p.amount} on:click={() => pickPreset(p)}>
          {p.label}
        </button>
      {/each}
      <button class="chip" class:on={custom} on:click={() => (custom = true)}>＋自定义</button>
    </div>

    {#if custom}
      <div class="units">
        {#each UNITS as u}
          <button class="unit" class:on={unit === u} on:click={() => setUnit(u)}>
            <b>{amount}</b><span>{UNIT_LABEL[u]}</span>
          </button>
        {/each}
      </div>
      <div class="stepper">
        <button on:click={() => changeAmount(-1)}>−</button>
        <b>{amount} {UNIT_LABEL[unit]}</b>
        <button on:click={() => changeAmount(1)}>＋</button>
      </div>
      <p class="hint">范围：分钟 1–59 · 小时 1–23 · 天 1–30（整数）</p>
    {/if}
  </section>

  <section>
    <p class="lbl">任务备注 · 目标 {GOAL} 字</p>
    <textarea
      rows="4"
      maxlength={HARD_LIMIT}
      placeholder="整理 Q3 复盘初稿…"
      bind:value={note}
    />
    <p class="cnt" class:goalMet>{chars} / {GOAL} 字{overHard ? '（已截断）' : ''}</p>
    <button class="go" disabled={!!active} on:click={start}>
      {active ? '番茄进行中…' : '▶ 开始专注'}
    </button>
  </section>

  <section>
    <p class="lbl">今日时间轴</p>
    {#if today.length === 0}
      <p class="empty">今天还没有记录</p>
    {:else}
      {#each today as s}
        <div class="tl">
          <span class="nm">{s.note || '（无备注）'}</span>
          <span class="bar" style="width:{barWidth(s)};opacity:{s.status === 'completed' ? 1 : 0.5}" />
          <span class="tm">{timeStr(s.actualSec ?? s.plannedSec)}</span>
        </div>
      {/each}
    {/if}
  </section>

  {#if settingsOpen && settings}
    <section class="settings">
      <p class="lbl">设置</p>
      <div class="row"><span>主题</span>
        <select value={settings.theme} on:change={setTheme}>
          <option value="system">跟随系统</option>
          <option value="dark">深色</option>
          <option value="light">浅色</option>
        </select>
      </div>
      <div class="row"><span>提示音</span><input type="checkbox" checked={settings.sound} on:change={toggleSound} /></div>
      <div class="row"><span>系统通知</span><input type="checkbox" checked={settings.notification} on:change={toggleNotify} /></div>
      <div class="row"><span>开机自启</span><input type="checkbox" checked={settings.autostart} on:change={setAutostart} /></div>
    </section>
  {/if}

  {#if statsOpen}
    <div class="overlay" role="presentation" on:click={() => (statsOpen = false)}>
      <div class="sheet" role="dialog" on:click|stopPropagation>
        <Stats />
      </div>
    </div>
  {/if}
</main>

<style>
  main { padding: 12px 14px 20px; max-width: 340px; margin: 0 auto; }
  header { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; }
  header b { color: var(--text); font-size: 14px; }
  .spacer { flex: 1; }
  .icon { background: var(--card); border: 1px solid var(--border); color: var(--text); border-radius: 8px; padding: 4px 8px; cursor: pointer; }
  .running { display: flex; align-items: center; justify-content: space-between; background: rgba(99, 102, 241, 0.15); border: 1px solid var(--accent); border-radius: 10px; padding: 10px 12px; margin-bottom: 12px; }
  .running i { display: block; font-style: normal; color: var(--muted); font-size: 11px; margin-top: 2px; }
  .danger { background: rgba(248, 113, 113, 0.15); color: #f87171; border: 1px solid rgba(248, 113, 113, 0.4); border-radius: 8px; padding: 6px 12px; cursor: pointer; }
  section { margin-bottom: 14px; }
  .lbl { font-size: 10px; color: var(--muted); text-transform: uppercase; letter-spacing: 0.5px; margin: 0 0 6px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { background: var(--card); border: 1px solid var(--border); color: var(--muted); border-radius: 999px; font-size: 12px; padding: 5px 12px; cursor: pointer; }
  .chip.on { border-color: var(--accent); color: var(--text); background: rgba(99, 102, 241, 0.15); }
  .units { display: flex; gap: 8px; margin: 10px 0; }
  .unit { flex: 1; background: var(--card); border: 1px solid var(--border); border-radius: 8px; padding: 8px 4px; text-align: center; cursor: pointer; color: var(--muted); }
  .unit.on { border-color: var(--accent); background: rgba(99, 102, 241, 0.15); color: var(--text); }
  .unit b { display: block; font: 700 18px/1.2 Consolas, monospace; }
  .stepper { display: flex; align-items: center; justify-content: space-between; background: var(--card); border: 1px solid var(--border); border-radius: 8px; padding: 4px 8px; }
  .stepper button { background: none; border: none; color: var(--accent); font-size: 16px; cursor: pointer; }
  .hint { font-size: 10px; color: var(--muted); margin: 6px 0 0; }
  textarea { width: 100%; box-sizing: border-box; background: var(--card); border: 1px solid var(--border); border-radius: 8px; color: var(--text); font: 13px/1.6 'Microsoft YaHei', sans-serif; padding: 8px; resize: vertical; }
  .cnt { font-size: 11px; color: var(--muted); text-align: right; margin: 4px 0 10px; }
  .cnt.goalMet { color: var(--ok); }
  .go { width: 100%; background: linear-gradient(90deg, #6366f1, #8b5cf6); color: #fff; border: none; border-radius: 8px; padding: 11px; font-size: 14px; font-weight: 600; cursor: pointer; }
  .go:disabled { opacity: 0.5; cursor: not-allowed; }
  .empty { color: var(--muted); font-size: 12px; }
  .tl { display: flex; align-items: center; gap: 8px; padding: 6px 0; border-bottom: 1px dashed var(--border); }
  .tl .nm { font-size: 12px; color: var(--text); width: 96px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tl .bar { height: 6px; border-radius: 3px; background: linear-gradient(90deg, var(--accent), var(--accent2)); }
  .tl .tm { font-size: 10px; color: var(--muted); margin-left: auto; font-family: Consolas, monospace; }
  .settings { background: var(--card); border: 1px solid var(--border); border-radius: 10px; padding: 10px 12px; }
  .row { display: flex; justify-content: space-between; align-items: center; padding: 6px 0; font-size: 13px; }
  .overlay { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.55); display: flex; align-items: center; justify-content: center; z-index: 10; }
  .sheet { background: var(--bg); border: 1px solid var(--border); border-radius: 12px; width: 320px; max-height: 90vh; overflow: auto; padding: 6px; }
</style>
