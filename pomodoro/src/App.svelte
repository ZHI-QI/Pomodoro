<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  const label = getCurrentWindow().label;
  if (label === 'ring') document.body.classList.add('ring-body');
</script>

<main>
  <p>窗口：{label}</p>
</main>
