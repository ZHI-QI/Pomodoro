<script lang="ts">
  /**
   * 全局统一线性风格图标（stroke = currentColor，继承文字颜色）
   * 用法：<Icon name="chart" size={16} />
   */
  type IconName =
    | 'chart'
    | 'settings'
    | 'close'
    | 'play'
    | 'stop'
    | 'check'
    | 'plus'
    | 'minus'
    | 'alert'
    | 'clock'
    | 'flame'
    | 'sound-on'
    | 'sound-off'
    | 'bell-on'
    | 'bell-off'
    | 'power';

  export let name: IconName;
  export let size = 16;

  const paths: Record<IconName, string> = {
    chart: 'M4 20V10M10 20V4M16 20v-7M20 20H2',
    settings:
      'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z M19.4 15a1.7 1.7 0 0 0 .34 1.87l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.7 1.7 0 0 0-1.87-.34 1.7 1.7 0 0 0-1.03 1.56V21a2 2 0 1 1-4 0v-.09a1.7 1.7 0 0 0-1.11-1.56 1.7 1.7 0 0 0-1.87.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.7 1.7 0 0 0 .34-1.87 1.7 1.7 0 0 0-1.56-1.03H3a2 2 0 1 1 0-4h.09a1.7 1.7 0 0 0 1.56-1.11 1.7 1.7 0 0 0-.34-1.87l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.7 1.7 0 0 0 1.87.34h.08a1.7 1.7 0 0 0 1.03-1.56V3a2 2 0 1 1 4 0v.09a1.7 1.7 0 0 0 1.03 1.56 1.7 1.7 0 0 0 1.87-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.7 1.7 0 0 0-.34 1.87v.08a1.7 1.7 0 0 0 1.56 1.03H21a2 2 0 1 1 0 4h-.09a1.7 1.7 0 0 0-1.56 1.03Z',
    close: 'M6 6l12 12M18 6L6 18',
    play: 'M8 5.5v13l11-6.5-11-6.5Z',
    stop: 'M7 5h10a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2Z',
    check: 'M4 12.5l5 5L20 6.5',
    plus: 'M12 5v14M5 12h14',
    minus: 'M5 12h14',
    alert: 'M12 3 2.5 20h19L12 3Z M12 10v4 M12 17.2v.1',
    clock: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z M12 7v5l3.5 2',
    flame:
      'M12 21c3.9 0 6.5-2.5 6.5-6 0-2.5-1.5-4.5-3-6-.4 1.2-1 2-2 2.5C13.5 9 13 6 10.5 3.5 10.5 7 8 8.5 6.8 10.6 6 12 5.5 13.4 5.5 15c0 3.5 2.6 6 6.5 6Z',
    'sound-on':
      'M4 9v6h3l5 4V5L7 9H4Z M15.5 8.5a5 5 0 0 1 0 7 M18 6a8.5 8.5 0 0 1 0 12',
    'sound-off': 'M4 9v6h3l5 4V5L7 9H4Z M16 9.5l5 5 M21 9.5l-5 5',
    'bell-on':
      'M6 9a6 6 0 1 1 12 0c0 4 1.5 5.5 2 6H4c.5-.5 2-2 2-6Z M10 19a2 2 0 0 0 4 0',
    'bell-off':
      'M6 9a6 6 0 0 1 9.3-5M18.6 7c.2.6.4 1.3.4 2 0 4 1.5 5.5 2 6H7M4 4l16 16 M10 19a2 2 0 0 0 4 0',
    power: 'M12 3v8 M6.3 6.3a8 8 0 1 0 11.4 0',
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.7"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#each paths[name].split(/(?=M)/) as d}
    <path {d} />
  {/each}
</svg>

<style>
  svg { display: inline-block; vertical-align: -0.15em; flex-shrink: 0; }
</style>
