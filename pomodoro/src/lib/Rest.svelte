<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';

  const log = (m: string) => invoke('debug_log', { msg: m }).catch(() => {});
  let logOnce = new Set<string>();
  const log1 = (k: string, m: string) => { if (!logOnce.has(k)) { logOnce.add(k); log(m); } };

  let remain = 60;
  let fading = false;
  let started = false;
  let canvas: HTMLCanvasElement;
  let stopTimer: (() => void) | undefined;

  interface Star { x: number; y: number; px: number; py: number; vx: number; vy: number; r: number; hue: number }
  interface Disk { ang: number; rad: number; speed: number; hue: number; size: number }

  // 倒计时仅在休息开始后启动；rest 窗口随应用启动即加载 WebView，
  // 若在挂载时启动倒计时，真正休息时早已倒数完毕、fading 将动画层隐去
  function beginCountdown() {
    remain = 60;
    fading = false;
    stopTimer?.();
    const iv = setInterval(() => {
      remain = Math.max(0, remain - 1);
      if (remain === 0) {
        fading = true;
        stopTimer?.();
      }
    }, 1000);
    stopTimer = () => clearInterval(iv);
  }

  onMount(() => {
    let offStart: (() => void) | undefined;
    let offFade: (() => void) | undefined;
    let cancelAnim: (() => void) | undefined;
    let resizeFn: (() => void) | undefined;
    let paintNow: (() => void) | undefined;
    let beginAll: (() => void) | undefined;

    (async () => {
      log(`Rest.svelte mounted (fix3) visibility=${document.visibilityState} size=${window.innerWidth}x${window.innerHeight}`);
      offStart = await listen('rest_start', () => {
        log('rest_start received');
        beginAll?.();
      });
      offFade = await listen('rest_fade', () => {
        log('rest_fade received');
        fading = true;
        stopTimer?.();
      });

      const ctx = canvas.getContext('2d');
      log(`getContext 2d: ${ctx ? 'ok' : 'NULL'}`);
      if (ctx) {
        const engine = startCanvas(ctx);
        resizeFn = engine.resize;
        paintNow = engine.paintNow;
        cancelAnim = engine.cancel;
      }

      beginAll = () => {
        if (started) return;
        started = true;
        log('beginAll: countdown + animation started');
        beginCountdown();
        // win.show() 是异步的，rest_start 发出时窗口可能尚未真正显示，
        // 稍后重取尺寸；心跳中也会持续校正 bitmap
        setTimeout(() => resizeFn?.(), 250);
        setTimeout(() => resizeFn?.(), 900);
      };

      // 心跳兜底（400ms）：
      // ① rest_start 丢失 → 窗口已可见却未启动 → 自动启动
      // ② 隐藏窗口期量到的尺寸为 0 / bitmap 与窗口不符 → 重铺
      // ③ 隐藏期 rAF 被冻结且显示后不恢复 → 强制同步帧（2.5fps 兜底，保证可见）
      const hb = setInterval(() => {
        const visible = document.visibilityState === 'visible';
        log1('hb', `heartbeat: visible=${visible} started=${started} fading=${fading}`);
        if (visible && !started) beginAll?.();
        if (started && !fading && visible) {
          resizeFn?.();
          paintNow?.();
        }
      }, 400);

      return;
    })();

    return () => {
      offStart?.();
      offFade?.();
      stopTimer?.();
      cancelAnim?.();
    };
  });

  function startCanvas(ctx: CanvasRenderingContext2D) {
    const c = ctx;
    const dpr = Math.min(2, window.devicePixelRatio || 1);
    let W = 0;
    let H = 0;
    let lastPaint = performance.now();

    const resize = () => {
      const nw = window.innerWidth;
      const nh = window.innerHeight;
      if (nw <= 0 || nh <= 0) return; // 窗口尚未真正显示，尺寸未知
      if (canvas.width === nw * dpr && canvas.height === nh * dpr) return; // 已最新，避免重铺闪烁
      W = nw;
      H = nh;
      canvas.width = W * dpr;
      canvas.height = H * dpr;
      c.setTransform(dpr, 0, 0, dpr, 0, 0);
      paintBase();
      log1('size', `resize: canvas bitmap ${canvas.width}x${canvas.height} (W=${W} H=${H} dpr=${dpr})`);
      if (!seeded) seed(); // 首次有效尺寸时初始化黑洞/星尘/吸积盘
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

    let t = Math.random() * 1000;
    let raf = 0;
    const G = 2600; // 引力常数（调参）
    const N = 420;
    let RH = 0;
    let stars: Star[] = [];
    let disks: Disk[] = [];
    let seeded = false;
    const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

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

    // 首次获得有效窗口尺寸时初始化粒子世界（隐藏期 W/H=0，半径无法计算）
    function seed() {
      RH = Math.min(W, H) * 0.055; // 事件视界半径
      stars = Array.from({ length: N }, () => {
        const s = { x: Math.random() * W, y: Math.random() * H, px: 0, py: 0, vx: 0, vy: 0, r: 0.6 + Math.random() * 1.6, hue: 200 + Math.random() * 80 };
        s.px = s.x; s.py = s.y;
        const dx = s.x - W / 2, dy = s.y - H / 2;
        const d = Math.hypot(dx, dy) || 1;
        const sp = 0.6 + Math.random() * 1.4;
        s.vx = (-dy / d) * sp;
        s.vy = (dx / d) * sp;
        return s;
      });
      disks = Array.from({ length: 150 }, () => ({
        ang: Math.random() * Math.PI * 2,
        rad: RH * (1.15 + Math.random() * 1.6),
        speed: 0.012 + 0.02 / (1 + Math.random() * 2),
        hue: 20 + Math.random() * 160,
        size: 0.8 + Math.random() * 1.8,
      }));
      seeded = true;
      log1('seed', `seed done: RH=${RH.toFixed(1)} stars=${stars.length} disks=${disks.length}`);
    }

    function draw() {
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
    }

    function frame() {
      if (W <= 0 || H <= 0) { raf = requestAnimationFrame(frame); return; }
      draw();
      log1('firstframe', 'first frame painted (rAF alive)');
      lastPaint = performance.now();
      raf = requestAnimationFrame(frame);
    }
    // 心跳兜底用的强制帧：rAF 若被冻结（隐藏窗口显示后未恢复），
    // 心跳每 400ms 调用一次，保证至少 ~2.5fps 可见动画
    function paintNow() {
      if (W <= 0 || H <= 0) return;
      if (performance.now() - lastPaint < 1100) return; // rAF 活着，不干预
      draw();
      log1('forced', 'paintNow FORCED frame (rAF stalled)');
      lastPaint = performance.now();
    }
    raf = requestAnimationFrame(frame);
    return {
      resize,
      paintNow,
      cancel: () => cancelAnimationFrame(raf),
    };
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
