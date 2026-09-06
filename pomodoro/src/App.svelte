<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Panel from './lib/Panel.svelte';
  import Ring from './lib/Ring.svelte';
  const label = getCurrentWindow().label;
  if (label === 'ring') document.body.classList.add('ring-body');
</script>

{#if label === 'ring'}
  <Ring />
{:else}
  <Panel />
{/if}
