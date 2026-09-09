<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Panel from './lib/Panel.svelte';
  import Rest from './lib/Rest.svelte';
  const label = getCurrentWindow().label;
</script>

{#if label === 'rest'}
  <Rest />
{:else}
  <Panel />
{/if}
