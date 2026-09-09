<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  let remain = 60;
  let fading = false;
  let canvas: HTMLCanvasElement;

  interface Star { x: number; y: number; px: number; py: number; vx: number; vy: number; r: number; hue: number }
  interface Disk { ang: number; rad: number; speed: number; hue: number; size: number }

  onMount(() => {
    let offStart: (() => void) | undefined;
    let offFade: (() => void) | undefined;
    let timer: ReturnType<typeof setInterval> | undefined;

    (async () => {
      offStart = await listen('rest_start', () => {
        remain = 60;
        fading = false;
      });
      offFade = await listen('rest_fade', () => (fading = true));

      timer = setInterval(() => {
        remain = Math.max(0, remain - 1);
        if (remain === 0) fading = true;
      }, 1000);

      startCanvas();
    })();

    return () => {
      offStart?.();
      offFade?.();
      if (timer) clearInterval(timer);
    };
  });

  function startCanvas() {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const c = ctx as CanvasRenderingContext2D;
    const dpr = Math.min(2, window.devicePixelRatio || 1);
    let W = window.innerWidth;
    let H = window.innerHeight;
    const resize = () => {
      W = window.innerWidth;
      H = window.innerHeight;
      canvas.width = W * dpr;
      canvas.height = H * dpr;
      c.setTransform(dpr, 0, 0, dpr, 0, 0);
      paintBase();
    };
    // 深空基底（不透明，一次性铺底）
    function paintBase() {
      const g = c.createRadialGradient(W / 2, H / 2, 0, W / 2, H / 2, Math.max(W, H) * 0.7);
      g.addColorStop(0, '#0b1030');
      g.addColorStop(0.55, '#070b1c');
      g.addColorStop(1, '#03040c');
      c.fillStyle = g;
      c.fillRect(0, 0, W, H);
      // 远景静态星
      for (let i = 0; i < 220; i++) {
        c.fillStyle = `rgba(200,215,255,${0.12 + Math.random() * 0.3})`;
        c.fillRect(Math.random() * W, Math.random() * H, 1, 1);
      }
    }
    resize();
    window.addEventListener('resize', resize);

    const RH = Math.min(W, H) * 0.055; // 事件视界半径
    const G = 2600; // 引力常数（调参）
    const N = 420;

    const respawn = (s: Star) => {
      const edge = Math.floor(Math.random() * 4);
      const m = 40;
      if (edge === 0) { s.x = Math.random() * W; s.y = -m; }
      else if (edge === 1) { s.x = W + m; s.y = Math.random() * H; }
      else if (edge === 2) { s.x = Math.random() * W; s.y = H + m; }
      else { s.x = -m; s.y = Math.random() * H; }
      // 切向初速 → 螺旋吸入
      const dx = s.x - W / 2, dy = s.y - H / 2;
      const d = Math.hypot(dx, dy) || 1;
      const sp = 0.6 + Math.random() * 1.4;
      s.vx = (-dy / d) * sp;
      s.vy = (dx / d) * sp;
      s.px = s.x; s.py = s.y;
      s.r = 0.6 + Math.random() * 1.6;
      s.hue = 200 + Math.random() * 80;
    };
    const stars: Star[] = Array.from({ length: N }, () => {
      const s = { x: 0, y: 0, px: 0, py: 0, vx: 0, vy: 0, r: 1, hue: 220 };
      respawn(s);
      // 初始随机撒在屏内
      s.x = Math.random() * W; s.y = Math.random() * H; s.px = s.x; s.py = s.y;
      return s;
    });

    const disks: Disk[] = Array.from({ length: 150 }, () => ({
      ang: Math.random() * Math.PI * 2,
      rad: RH * (1.15 + Math.random() * 1.6),
      speed: 0.012 + 0.02 / (1 + Math.random() * 2),
      hue: 20 + Math.random() * 160,
      size: 0.8 + Math.random() * 1.8,
    }));

    let t = Math.random() * 1000;
    let raf = 0;
    const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    function frame() {
      t += reduce ? 0 : 0.16;
      // 黑洞游荡：慢速 Lissajous
      const bx = W / 2 + Math.sin(t * 0.011 + 1.2) * W * 0.27;
      const by = H / 2 + Math.sin(t * 0.007 + 0.4) * H * 0.2;

      // 半透明拖尾（保留吸积残影）
      c.fillStyle = 'rgba(4,6,16,0.32)';
      c.fillRect(0, 0, W, H);

      // 星尘引力
      for (const s of stars) {
        const dx = bx - s.x, dy = by - s.y;
        const d2 = Math.max(dx * dx + dy * dy, RH * RH * 0.25);
        const d = Math.sqrt(d2);
        const a = G / d2;
        s.vx += (dx / d) * a;
        s.vy += (dy / d) * a;
        s.px = s.x; s.py = s.y;
        s.x += s.vx * (reduce ? 0.3 : 1);
        s.y += s.vy * (reduce ? 0.3 : 1);
        if (d < RH * 0.9 || s.x < -80 || s.x > W + 80 || s.y < -80 || s.y > H + 80) {
          respawn(s);
          continue;
        }
        const stretch = Math.min(14, Math.hypot(s.vx, s.vy) * 1.6);
        c.strokeStyle = `hsla(${s.hue}, 85%, ${68 + stretch}%, 0.75)`;
        c.lineWidth = s.r;
        c.beginPath();
        c.moveTo(s.px, s.py);
        c.lineTo(s.x - (s.vx / (Math.hypot(s.vx, s.vy) || 1)) * stretch * 0.4,
                 s.y - (s.vy / (Math.hypot(s.vx, s.vy) || 1)) * stretch * 0.4);
        c.stroke();
      }

      // 吸积盘
      for (const p of disks) {
        p.ang += p.speed * (RH * 1.4 / p.rad);
        p.rad -= 0.06;
        if (p.rad < RH * 1.02) { p.rad = RH * (2.4 + Math.random() * 0.6); p.ang = Math.random() * Math.PI * 2; }
        const x = bx + Math.cos(p.ang) * p.rad;
        const y = by + Math.sin(p.ang) * p.rad * 0.42; // 椭圆透视
        c.fillStyle = `hsla(${p.hue}, 95%, 65%, 0.8)`;
        c.beginPath();
        c.arc(x, y, p.size, 0, Math.PI * 2);
        c.fill();
      }

      // 光子环
      c.save();
      c.shadowColor = 'rgba(255,190,120,0.9)';
      c.shadowBlur = 24;
      c.strokeStyle = 'rgba(255,214,150,0.95)';
      c.lineWidth = 2.2;
      c.beginPath();
      c.ellipse(bx, by, RH * 1.08, RH * 1.08, 0, 0, Math.PI * 2);
      c.stroke();
      c.restore();

      // 黑洞本体
      const core = c.createRadialGradient(bx, by, RH * 0.1, bx, by, RH);
      core.addColorStop(0, '#000');
      core.addColorStop(0.82, '#000');
      core.addColorStop(1, 'rgba(10,6,20,0.4)');
      c.fillStyle = core;
      c.beginPath();
      c.arc(bx, by, RH, 0, Math.PI * 2);
      c.fill();

      raf = requestAnimationFrame(frame);
    }
    raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(raf);
  }
</script>

<div class="rest" class:fading>
  <canvas bind:this={canvas} />
  <p class="tip">黑洞休憩中 · 剩余 {remain}s</p>
</div>

<style>
  :global(html), :global(body) {
    overflow: hidden;
    background: #04060f;
  }
  .rest {
    position: fixed;
    inset: 0;
    background: #04060f;
    overflow: hidden;
    transition: opacity 1.6s ease;
    user-select: none;
  }
  .rest.fading { opacity: 0; }
  canvas { display: block; width: 100vw; height: 100vh; }
  .tip {
    position: fixed;
    bottom: 4vh;
    left: 50%;
    transform: translateX(-50%);
    margin: 0;
    padding: 8px 18px;
    border-radius: 999px;
    font: 13px 'Microsoft YaHei', sans-serif;
    letter-spacing: 2px;
    color: #9aa7c7;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.09);
    -webkit-backdrop-filter: blur(8px);
    backdrop-filter: blur(8px);
  }
</style>
