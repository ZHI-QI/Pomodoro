<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { currentMonitor, getCurrentWindow } from '@tauri-apps/api/window';
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  import Icon from './Icon.svelte';
  import { getActiveSession, type AbortDto, type TickDto } from './ipc';

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
        await listen<{ sound: boolean }>('session_done', (e) => {
          running = false;
          done = true;
          remaining = 0;
          if (e.payload.sound) new Audio('/ding.wav').play().catch(() => {});
        }),
        await listen<AbortDto>('session_aborted', () => {
          running = false;
          done = false;
          remaining = 0;
          planned = 0;
          noteShort = '';
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
      <defs>
        <linearGradient id="ringGrad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#22d3ee" />
          <stop offset="55%" stop-color="#6366f1" />
          <stop offset="100%" stop-color="#a78bfa" />
        </linearGradient>
      </defs>
      <circle cx="48" cy="48" r={R} class="track" />
      <circle
        cx="48" cy="48" r={R}
        class="arc"
        stroke="url(#ringGrad)"
        stroke-dasharray={C}
        stroke-dashoffset={C * (1 - progress)}
      />
    </svg>
    <div class="center">
      <b>{timeText}</b>
      <i>{noteShort}</i>
    </div>
  {:else if done}
    <span class="done-ic"><Icon name="check" size={26} /></span>
  {:else}
    <span class="plus-ic"><Icon name="plus" size={26} /></span>
  {/if}
</div>

<style>
  .ring {
    width: 96px;
    height: 96px;
    border-radius: 50%;
    position: relative;
    cursor: pointer;
    transition: opacity 0.25s, transform 0.2s;
    background:
      radial-gradient(circle at 32% 28%, rgba(255, 255, 255, 0.16), transparent 52%),
      linear-gradient(160deg, rgba(20, 28, 56, 0.88), rgba(10, 14, 32, 0.82));
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.14),
      0 6px 24px rgba(2, 6, 23, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .ring.docked { opacity: 0.2; }
  .ring.docked:hover { opacity: 1; }
  .ring:hover { transform: scale(1.08); }
  .track { fill: none; stroke: rgba(255, 255, 255, 0.09); stroke-width: 6; }
  .arc {
    fill: none;
    stroke-width: 6;
    stroke-linecap: round;
    filter: drop-shadow(0 0 5px rgba(99, 102, 241, 0.8));
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
  .center b { font: 700 15px/1 Consolas, monospace; color: #e8edf7; font-variant-numeric: tabular-nums; }
  .center i {
    font: 10px/1.4 'Microsoft YaHei', sans-serif; font-style: normal; color: #8b96ad;
    max-width: 72px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .plus-ic, .done-ic { display: flex; align-items: center; justify-content: center; }
  .plus-ic { color: rgba(139, 150, 173, 0.85); }
  .ring:hover .plus-ic { color: #22d3ee; }
  .done-ic { color: #34d399; filter: drop-shadow(0 0 8px rgba(52, 211, 153, 0.6)); }
  .pulse { animation: pulse 1.2s ease-out infinite; }
  @keyframes pulse {
    0% { box-shadow: inset 0 1px 0 rgba(255,255,255,0.14), 0 6px 24px rgba(2,6,23,0.55), 0 0 0 0 rgba(52, 211, 153, 0.5); }
    100% { box-shadow: inset 0 1px 0 rgba(255,255,255,0.14), 0 6px 24px rgba(2,6,23,0.55), 0 0 0 18px rgba(52, 211, 153, 0); }
  }
</style>
