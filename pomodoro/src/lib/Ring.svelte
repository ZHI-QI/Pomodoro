<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { currentMonitor, getCurrentWindow } from '@tauri-apps/api/window';
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { getActiveSession, type TickDto } from './ipc';

  let remaining = 0;
  let planned = 0;
  let noteShort = '';
  let running = false;
  let done = false;
  let docked = false;
  const R = 40;
  const C = 2 * Math.PI * R;

  $: progress = planned > 0 ? remaining / planned : 0;
  $: timeText =
    remaining >= 3600
      ? `${Math.floor(remaining / 3600)}:${String(Math.floor((remaining % 3600) / 60)).padStart(2, '0')}:${String(remaining % 60).padStart(2, '0')}`
      : `${String(Math.floor(remaining / 60)).padStart(2, '0')}:${String(remaining % 60).padStart(2, '0')}`;

  function fmt(s: TickDto) {
    remaining = s.remainingSec;
    planned = s.plannedSec;
    noteShort = s.noteShort;
    running = true;
    done = false;
  }

  async function openPanel() {
    done = false;
    const panel = await WebviewWindow.getByLabel('panel');
    await panel?.show();
    await panel?.setFocus();
  }

  onMount(() => {
    const offs: (() => void)[] = [];
    (async () => {
      offs.push(
        await listen<TickDto>('tick', (e) => fmt(e.payload)),
        await listen('session_done', () => {
          running = false;
          done = true;
          remaining = 0;
          new Audio('/ding.wav').play().catch(() => {});
        })
      );
      const active = await getActiveSession();
      if (active) {
        running = true;
        planned = active.plannedSec;
        remaining = active.plannedSec;
        noteShort = active.note.slice(0, 8);
      }
      const win = getCurrentWindow();
      offs.push(
        await win.onMoved(async () => {
          const pos = await win.outerPosition();
          const mon = await currentMonitor();
          docked =
            !!mon &&
            (pos.x <= 8 ||
              pos.y <= 8 ||
              pos.x + 96 >= mon.size.width - 8 ||
              pos.y + 96 >= mon.size.height - 8);
        })
      );
    })();
    return () => offs.forEach((f) => f());
  });
</script>

<div
  class="ring"
  class:docked
  class:pulse={done}
  data-tauri-drag-region
  on:click={openPanel}
  role="button"
  tabindex="0"
>
  {#if running}
    <svg viewBox="0 0 96 96" width="96" height="96">
      <circle cx="48" cy="48" r={R} class="track" />
      <circle
        cx="48"
        cy="48"
        r={R}
        class="arc"
        stroke-dasharray={C}
        stroke-dashoffset={C * (1 - progress)}
      />
    </svg>
    <div class="center">
      <b>{timeText}</b>
      <i>{noteShort}</i>
    </div>
  {:else if done}
    <div class="center ok"><b>✔ 完成</b></div>
  {:else}
    <div class="center"><b>＋</b></div>
  {/if}
</div>

<style>
  .ring {
    width: 96px;
    height: 96px;
    border-radius: 50%;
    position: relative;
    cursor: pointer;
    transition: opacity 0.2s;
    background: transparent;
  }
  .ring.docked { opacity: 0.2; }
  .ring.docked:hover { opacity: 1; }
  .ring:hover { transform: scale(1.08); }
  .track { fill: rgba(15, 23, 42, 0.88); stroke: rgba(255, 255, 255, 0.1); stroke-width: 6; }
  .arc {
    fill: none;
    stroke: var(--accent);
    stroke-width: 6;
    stroke-linecap: round;
    transform: rotate(-90deg);
    transform-origin: center;
    transition: stroke-dashoffset 1s linear;
  }
  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }
  .center b { font: 700 15px/1 Consolas, monospace; color: var(--text); }
  .center i { font: 10px/1.4 'Microsoft YaHei', sans-serif; font-style: normal; color: var(--muted); max-width: 72px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ok b { color: var(--ok); }
  .pulse { animation: pulse 1.2s ease-out infinite; }
  @keyframes pulse {
    0% { box-shadow: 0 0 0 0 rgba(52, 211, 153, 0.5); }
    100% { box-shadow: 0 0 0 18px rgba(52, 211, 153, 0); }
  }
</style>
