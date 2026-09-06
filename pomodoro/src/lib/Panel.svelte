<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { listen } from '@tauri-apps/api/event';
  import Stats from './Stats.svelte';
  import Icon from './Icon.svelte';
  import { clampAmount, LIMITS, outcomeBanner, PRESETS, UNIT_LABEL, UNITS, type Unit } from './panel';
  import { GOAL, HARD_LIMIT, noteLen } from './noteCounter';
  import {
    abortSession,
    getActiveSession,
    getSettings,
    getStoreOutcome,
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
  let banner: string | null = null;

  $: chars = noteLen(note);
  $: goalMet = chars >= GOAL;
  $: overHard = chars > HARD_LIMIT;
  $: focusProgress = active && active.plannedSec > 0 ? 1 - activeRemaining / active.plannedSec : 0;

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
    const pct = Math.min(100, Math.max(10, (s.plannedSec / 3600) * 100));
    return `${pct}%`;
  }

  function timeStr(sec: number): string {
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    return h > 0 ? `${h}小时${m}分` : `${m}分钟`;
  }

  function mmss(sec: number): string {
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    const s = sec % 60;
    const mm = String(m).padStart(2, '0');
    const ss = String(s).padStart(2, '0');
    return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
  }

  onMount(() => {
    const offs: (() => void)[] = [];
    (async () => {
      settings = await getSettings();
      await initTheme(settings.theme);
      banner = outcomeBanner((await getStoreOutcome()).outcome);
      await refresh();
      offs.push(
        await listen<{ remainingSec: number; sessionId: number }>('tick', (e) => {
          if (active && e.payload.sessionId === active.id) activeRemaining = e.payload.remainingSec;
        }),
        await listen('session_done', refresh)
      );
    })();
    return () => offs.forEach((f) => f());
  });
</script>

<svelte:window on:keydown={(e) => e.key === 'Escape' && (statsOpen = false)} />

<main>
  {#if banner}
    <div class="banner glass" role="status" transition:fly={{ y: -12, duration: 260 }}>
      <span><Icon name="alert" size={14} /> {banner}</span>
      <button class="ghost-ic" title="关闭" on:click={() => (banner = null)}><Icon name="close" size={13} /></button>
    </div>
  {/if}

  <header>
    <span class="brand"><Icon name="flame" size={17} /> <b>番茄钟</b></span>
    <span class="spacer" />
    <button class="ghost-ic" title="统计" on:click={() => (statsOpen = true)}><Icon name="chart" size={16} /></button>
    <button class="ghost-ic" title="设置" on:click={() => (settingsOpen = !settingsOpen)}>
      <Icon name="settings" size={16} />
    </button>
  </header>

  {#if active}
    <!-- ═══ 专注视图：启动后平滑接管全屏 ═══ -->
    <section class="focus glass" in:fly={{ y: 26, duration: 420 }} out:fade={{ duration: 200 }}>
      <div class="ring-wrap">
        <svg viewBox="0 0 200 200" class="dial">
          <defs>
            <linearGradient id="focusGrad" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stop-color="var(--accent2)" />
              <stop offset="55%" stop-color="var(--accent)" />
              <stop offset="100%" stop-color="var(--accent3)" />
            </linearGradient>
          </defs>
          <circle class="dial-track" cx="100" cy="100" r="88" />
          <circle
            class="dial-arc"
            cx="100" cy="100" r="88"
            stroke-dasharray={2 * Math.PI * 88}
            stroke-dashoffset={2 * Math.PI * 88 * (1 - focusProgress)}
          />
        </svg>
        <div class="dial-center">
          <span class="clock-ic"><Icon name="clock" size={15} /></span>
          <b class="tabular">{mmss(activeRemaining)}</b>
          <i>剩余时间</i>
        </div>
      </div>
      <p class="focus-note">{active.note || '（无备注）'}</p>
      <p class="focus-meta">计划 {timeStr(active.plannedSec)} · 已专注 {timeStr(active.plannedSec - activeRemaining)}</p>
      <button class="abandon" on:click={giveUp}>
        <Icon name="stop" size={15} /> 放弃专注
      </button>
    </section>
  {:else}
    <!-- ═══ 初始视图：时长 + 备注 + 时间轴 ═══ -->
    <div class="idle" in:fade={{ duration: 320, delay: 120 }}>
      <section class="card glass">
        <p class="lbl">快速时长</p>
        <div class="chips">
          {#each PRESETS as p}
            <button
              class="chip"
              class:on={!custom && unit === p.unit && amount === p.amount}
              on:click={() => pickPreset(p)}
            >
              {p.label}
            </button>
          {/each}
          <button class="chip" class:on={custom} on:click={() => (custom = true)}>
            <Icon name="plus" size={11} /> 自定义
          </button>
        </div>

        {#if custom}
          <div class="units">
            {#each UNITS as u}
              <button class="unit" class:on={unit === u} on:click={() => setUnit(u)}>
                <b class="tabular">{amount}</b><span>{UNIT_LABEL[u]}</span>
              </button>
            {/each}
          </div>
          <div class="stepper glass">
            <button class="ghost-ic" aria-label="减少" on:click={() => changeAmount(-1)}><Icon name="minus" size={14} /></button>
            <b class="tabular">{amount} {UNIT_LABEL[unit]}</b>
            <button class="ghost-ic" aria-label="增加" on:click={() => changeAmount(1)}><Icon name="plus" size={14} /></button>
          </div>
          <p class="hint">范围：{UNITS.map((u) => `${UNIT_LABEL[u]} ${LIMITS[u][0]}–${LIMITS[u][1]}`).join(' · ')}（整数）</p>
        {/if}
      </section>

      <section class="card glass">
        <p class="lbl">任务备注 · 目标 {GOAL} 字</p>
        <textarea
          rows="4"
          maxlength={HARD_LIMIT}
          placeholder="整理 Q3 复盘初稿…"
          bind:value={note}
        />
        <p class="cnt" class:goalMet>{chars} / {GOAL} 字{overHard ? '（已截断）' : ''}</p>
        <button class="go" on:click={start}>
          <Icon name="play" size={15} /> 开始专注
        </button>
      </section>

      <section class="card glass">
        <p class="lbl">今日时间轴</p>
        {#if today.length === 0}
          <p class="empty">今天还没有记录</p>
        {:else}
          {#each today as s}
            <div class="tl">
              <span class="nm">{s.note || '（无备注）'}</span>
              <span class="bar-track"><span class="bar" style="width:{barWidth(s)};opacity:{s.status === 'completed' ? 1 : 0.45}" /></span>
              <span class="tm mono">{timeStr(s.actualSec ?? s.plannedSec)}</span>
            </div>
          {/each}
        {/if}
      </section>

      {#if settingsOpen && settings}
        <section class="card glass settings" transition:fly={{ y: 14, duration: 240 }}>
          <p class="lbl">设置</p>
          <div class="row"><span>主题</span>
            <select value={settings.theme} on:change={setTheme}>
              <option value="system">跟随系统</option>
              <option value="dark">深色</option>
              <option value="light">浅色</option>
            </select>
          </div>
          <div class="row">
            <span><Icon name={settings.sound ? 'sound-on' : 'sound-off'} size={14} /> 提示音</span>
            <input type="checkbox" checked={settings.sound} on:change={toggleSound} />
          </div>
          <div class="row">
            <span><Icon name={settings.notification ? 'bell-on' : 'bell-off'} size={14} /> 系统通知</span>
            <input type="checkbox" checked={settings.notification} on:change={toggleNotify} />
          </div>
          <div class="row">
            <span><Icon name="power" size={14} /> 开机自启</span>
            <input type="checkbox" checked={settings.autostart} on:change={setAutostart} />
          </div>
        </section>
      {/if}
    </div>
  {/if}

  {#if statsOpen}
    <div
      class="overlay"
      role="button"
      tabindex="-1"
      aria-label="关闭统计"
      transition:fade={{ duration: 180 }}
      on:click={(e) => e.target === e.currentTarget && (statsOpen = false)}
      on:keydown={(e) => e.key === 'Escape' && (statsOpen = false)}
    >
      <div class="sheet glass" role="dialog" aria-modal="true">
        <Stats />
      </div>
    </div>
  {/if}
</main>

<style>
  main {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 520px;
    margin: 0 auto;
    padding: clamp(10px, 2.5vw, 18px) clamp(12px, 3vw, 20px) 24px;
  }

  header { display: flex; align-items: center; gap: 8px; margin-bottom: 12px; }
  .brand { display: inline-flex; align-items: center; gap: 6px; color: var(--accent2); }
  .brand b { color: var(--text); font-size: clamp(13px, 3.4vw, 15px); letter-spacing: 0.5px; }
  .spacer { flex: 1; }

  .ghost-ic {
    display: inline-flex; align-items: center; justify-content: center; gap: 4px;
    background: var(--glass); border: 1px solid var(--glass-border);
    color: var(--muted); border-radius: 9px;
    padding: 6px; cursor: pointer;
    transition: color 0.2s, border-color 0.2s, transform 0.15s;
  }
  .ghost-ic:hover { color: var(--accent2); border-color: rgba(34, 211, 238, 0.4); transform: translateY(-1px); }

  .banner {
    display: flex; align-items: center; justify-content: space-between; gap: 8px;
    padding: 8px 10px; margin-bottom: 10px; font-size: 12px; color: var(--warn);
  }
  .banner span { display: inline-flex; align-items: center; gap: 6px; }

  section.card { padding: clamp(10px, 2.6vw, 14px); margin-bottom: 12px; }

  .lbl {
    font-size: 10px; color: var(--muted); text-transform: uppercase;
    letter-spacing: 1.2px; margin: 0 0 8px; font-weight: 600;
  }

  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip {
    display: inline-flex; align-items: center; gap: 4px;
    background: var(--glass); border: 1px solid var(--glass-border);
    color: var(--muted); border-radius: 999px;
    font-size: 12px; padding: 5px 12px; cursor: pointer;
    transition: all 0.2s;
  }
  .chip.on {
    border-color: transparent; color: #fff;
    background: linear-gradient(120deg, var(--accent), var(--accent3));
    box-shadow: 0 2px 14px rgba(99, 102, 241, 0.45);
  }

  .units { display: flex; gap: 8px; margin: 10px 0; }
  .unit {
    flex: 1; background: var(--glass); border: 1px solid var(--glass-border);
    border-radius: 10px; padding: 8px 4px; text-align: center;
    cursor: pointer; color: var(--muted); transition: all 0.2s;
  }
  .unit.on {
    border-color: rgba(34, 211, 238, 0.5);
    background: rgba(34, 211, 238, 0.10); color: var(--text);
    box-shadow: 0 0 18px rgba(34, 211, 238, 0.18) inset;
  }
  .unit b { display: block; font: 700 clamp(16px, 4vw, 18px)/1.2 Consolas, monospace; }

  .stepper {
    display: flex; align-items: center; justify-content: space-between;
    padding: 4px 8px; margin-bottom: 6px;
  }
  .stepper b { font: 700 14px Consolas, monospace; }
  .hint { font-size: 10px; color: var(--muted); margin: 6px 0 0; }

  textarea {
    width: 100%; background: var(--glass);
    border: 1px solid var(--glass-border); border-radius: 10px;
    color: var(--text); font: 13px/1.7 'Microsoft YaHei', sans-serif;
    padding: 9px; resize: vertical; transition: border-color 0.2s;
  }
  textarea:focus { border-color: rgba(99, 102, 241, 0.55); }
  .cnt { font-size: 11px; color: var(--muted); text-align: right; margin: 4px 0 10px; }
  .cnt.goalMet { color: var(--ok); }

  .go {
    width: 100%; display: inline-flex; align-items: center; justify-content: center; gap: 7px;
    background: linear-gradient(115deg, var(--accent2) 0%, var(--accent) 55%, var(--accent3) 100%);
    color: #fff; border: none; border-radius: 11px;
    padding: 12px; font-size: clamp(13px, 3.4vw, 14px); font-weight: 700; letter-spacing: 2px;
    cursor: pointer; transition: transform 0.15s, box-shadow 0.25s, filter 0.2s;
    box-shadow: 0 4px 22px rgba(99, 102, 241, 0.45);
  }
  .go:hover { transform: translateY(-1px); box-shadow: 0 6px 30px rgba(99, 102, 241, 0.6); filter: brightness(1.08); }
  .go:active { transform: translateY(0); }

  .empty { color: var(--muted); font-size: 12px; margin: 2px 0; }

  .tl { display: flex; align-items: center; gap: 8px; padding: 6px 0; border-bottom: 1px dashed var(--glass-border); }
  .tl:last-child { border-bottom: none; }
  .tl .nm { font-size: 12px; color: var(--text); width: 92px; flex-shrink: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bar-track { flex: 1; height: 6px; border-radius: 3px; background: rgba(139, 150, 173, 0.15); overflow: hidden; }
  .bar {
    display: block; height: 100%; border-radius: 3px;
    background: linear-gradient(90deg, var(--accent2), var(--accent), var(--accent3));
    box-shadow: 0 0 8px rgba(99, 102, 241, 0.5);
    transition: width 0.4s ease;
  }
  .tl .tm { font-size: 10px; color: var(--muted); flex-shrink: 0; }

  .settings .row { display: flex; justify-content: space-between; align-items: center; padding: 7px 0; font-size: 13px; }
  .settings .row > span:first-child { display: inline-flex; align-items: center; gap: 6px; color: var(--text); }
  .settings select { padding: 5px 8px; font-size: 12px; }
  .settings input[type='checkbox'] { accent-color: var(--accent); width: 16px; height: 16px; cursor: pointer; }

  /* ═══ 专注视图 ═══ */
  .focus {
    display: flex; flex-direction: column; align-items: center;
    padding: clamp(18px, 5vw, 30px) 16px 22px; margin-top: 6px;
  }
  .ring-wrap { position: relative; width: clamp(180px, 62vw, 230px); aspect-ratio: 1; }
  .dial { width: 100%; height: 100%; transform: rotate(-90deg); }
  .dial-track { fill: none; stroke: rgba(139, 150, 173, 0.16); stroke-width: 9; }
  .dial-arc {
    fill: none; stroke: url(#focusGrad); stroke-width: 9; stroke-linecap: round;
    filter: drop-shadow(0 0 10px rgba(99, 102, 241, 0.65));
    transition: stroke-dashoffset 1s linear;
  }
  .dial-center {
    position: absolute; inset: 0; display: flex; flex-direction: column;
    align-items: center; justify-content: center; gap: 2px; pointer-events: none;
  }
  .clock-ic { color: var(--accent2); opacity: 0.85; }
  .dial-center b { font: 700 clamp(26px, 8vw, 34px)/1.1 Consolas, monospace; color: var(--text); letter-spacing: 1px; }
  .dial-center i { font-style: normal; font-size: 10px; color: var(--muted); letter-spacing: 2px; }

  .focus-note {
    margin: 16px 0 2px; font-size: clamp(13px, 3.4vw, 14px); color: var(--text);
    max-width: 90%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-align: center;
  }
  .focus-meta { margin: 0 0 18px; font-size: 11px; color: var(--muted); }

  .abandon {
    display: inline-flex; align-items: center; gap: 6px;
    background: rgba(248, 113, 113, 0.10); color: var(--danger);
    border: 1px solid rgba(248, 113, 113, 0.35); border-radius: 10px;
    padding: 8px 18px; font-size: 13px; cursor: pointer;
    transition: background 0.2s, box-shadow 0.2s;
  }
  .abandon:hover { background: rgba(248, 113, 113, 0.2); box-shadow: 0 0 18px rgba(248, 113, 113, 0.25); }

  /* ═══ 统计弹窗 ═══ */
  .overlay {
    position: fixed; inset: 0; z-index: 10;
    background: rgba(4, 8, 20, 0.45);
    -webkit-backdrop-filter: blur(6px); backdrop-filter: blur(6px);
    display: flex; align-items: center; justify-content: center;
  }
  .sheet {
    width: min(340px, calc(100vw - 32px));
    max-height: min(86vh, 560px); overflow: auto;
    padding: 8px;
    animation: sheet-in 0.28s cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes sheet-in {
    from { opacity: 0; transform: translateY(18px) scale(0.97); }
    to { opacity: 1; transform: none; }
  }
</style>
