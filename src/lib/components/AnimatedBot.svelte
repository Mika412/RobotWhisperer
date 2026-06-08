<script lang="ts">
  let {
    size = 64,
    strokeWidth = 1.5,
    class: className = "",
    interactive = false,
    readArea,
    bounds,
  }: {
    size?: number;
    strokeWidth?: number;
    class?: string;
    interactive?: boolean;
    readArea?: () => DOMRect | undefined;
    bounds?: () => DOMRect | undefined;
  } = $props();

  let wrap = $state<HTMLSpanElement>();
  let headEl: SVGGElement | undefined = $state();
  let eyesEl: SVGGElement | undefined = $state();
  let antennaEl: SVGGElement | undefined = $state();
  let blink = $state(false);

  const ACTIONS = ["loop8", "loop", "barrelroll", "frontflip", "hop", "read", "read"];
  const DUR: Record<string, number> = {
    loop8: 2600,
    loop: 2200,
    barrelroll: 1200,
    frontflip: 1300,
    hop: 800,
    read: 3400,
  };
  const FLEE_R = 80;
  const LOOK_R = 240;
  const EYE_MAX = 1.6;
  const HEAD_MAX = 6;
  const MAX_R = 130;

  type Pose = {
    x: number;
    y: number;
    rx: number;
    ry: number;
    rz: number;
    sc: number;
    reading: boolean;
  };
  type Action = {
    type: string;
    start: number;
    dur: number;
    tx: number;
    ty: number;
    rcx: number;
    rcy: number;
    fit: number;
  };

  function rand(min: number, max: number): number {
    return min + Math.random() * (max - min);
  }
  function clamp(v: number, lo: number, hi: number): number {
    return Math.min(hi, Math.max(lo, v));
  }
  function smooth(t: number): number {
    return t * t * (3 - 2 * t);
  }

  function pose(a: Action, now: number): Pose {
    const u = clamp((now - a.start) / a.dur, 0, 1);
    const e = smooth(u);
    const th = 2 * Math.PI * e;
    const base = { rx: 0, ry: 0, rz: 0, sc: 1, reading: false };
    if (a.type === "loop8")
      return { ...base, x: 44 * Math.sin(th), y: 17 * Math.sin(2 * th), rz: 360 * e };
    if (a.type === "loop")
      return { ...base, x: 34 * Math.sin(th), y: -34 + 34 * Math.cos(th), rz: -360 * e };
    if (a.type === "barrelroll")
      return { ...base, x: 0, y: -8 * Math.sin(Math.PI * e), ry: 360 * e };
    if (a.type === "frontflip")
      return { ...base, x: 0, y: -16 * Math.sin(Math.PI * e), rx: -360 * e };
    if (a.type === "hop")
      return {
        ...base,
        x: 0,
        y: -14 * Math.sin(Math.PI * e),
        sc: 1 - 0.12 * Math.sin(Math.PI * e),
      };
    const p1 = 0.28;
    const p2 = 0.72;
    if (u < p1) {
      const b = smooth(u / p1);
      return { ...base, x: a.tx * b, y: a.ty * b };
    }
    if (u < p2) return { ...base, x: a.tx, y: a.ty, reading: true };
    const b = smooth((u - p2) / (1 - p2));
    return { ...base, x: a.tx * (1 - b), y: a.ty * (1 - b) };
  }

  $effect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    const timers: ReturnType<typeof setTimeout>[] = [];
    const later = (fn: () => void, ms: number) => {
      const id = setTimeout(fn, ms);
      timers.push(id);
      return id;
    };

    if (!interactive) {
      const doBlink = () => {
        blink = true;
        later(() => {
          blink = false;
          if (Math.random() < 0.3) {
            later(() => {
              blink = true;
              later(() => (blink = false), 100);
            }, 150);
          }
        }, 100);
      };
      const loopBlink = () => later(() => (doBlink(), loopBlink()), rand(2600, 6200));
      loopBlink();
      return () => timers.forEach(clearTimeout);
    }

    let px = 0;
    let py = 0;
    let vx = 0;
    let vy = 0;
    let prevPx = 0;
    let gx = 0;
    let gy = 0;
    let hr = 0;
    let bs = 1;
    let aAng = 0;
    let aTarget = 0;
    let restCx = 0;
    let restCy = 0;
    let mouseX = 0;
    let mouseY = 0;
    let mousePresent = false;
    let fleeing = false;
    let blinkClosed = false;
    let action: Action | null = null;
    let raf = 0;
    let last = performance.now();

    const measure = () => {
      if (!wrap) return;
      const r = wrap.getBoundingClientRect();
      restCx = r.left + r.width / 2 - px;
      restCy = r.top + r.height / 2 - py;
    };
    const onMove = (e: PointerEvent) => {
      if (e.pointerType !== "mouse") return;
      mouseX = e.clientX;
      mouseY = e.clientY;
      mousePresent = true;
    };
    const onLeave = () => (mousePresent = false);

    const doBlink = () => {
      blinkClosed = true;
      later(() => {
        blinkClosed = false;
        if (Math.random() < 0.3) {
          later(() => {
            blinkClosed = true;
            later(() => (blinkClosed = false), 90);
          }, 150);
        }
      }, 95);
    };
    const loopBlink = () => later(() => (doBlink(), loopBlink()), rand(2600, 6200));

    const loopAntenna = () =>
      later(
        () => {
          aTarget = rand(9, 15) * (Math.random() < 0.5 ? 1 : -1);
          later(() => (aTarget = 0), rand(900, 1500));
          loopAntenna();
        },
        rand(3500, 8000),
      );

    const readEdgeTarget = (a: Action) => {
      const rect = readArea?.();
      if (!rect) {
        a.tx = rand(-50, 50);
        a.ty = -42;
        a.rcx = restCx + a.tx;
        a.rcy = restCy;
        return;
      }
      const margin = size * 0.36;
      a.tx = rand(rect.left + 8, rect.right - 8) - restCx;
      a.ty = rect.top - margin - restCy;
      a.rcx = a.tx + restCx;
      a.rcy = rect.top + rect.height / 2;
    };

    const fitScale = () => {
      const b = bounds?.();
      if (!b) return 1;
      const half = size / 2 + 4;
      const avail = Math.min(
        restCy - b.top - half,
        b.bottom - restCy - half,
        restCx - b.left - half,
        b.right - restCx - half,
      );
      return clamp(avail / 64, 0.15, 1);
    };

    const loopAction = () =>
      later(
        () => {
          if (!action && !fleeing && Math.hypot(px, py) < 6) {
            const type = ACTIONS[Math.floor(Math.random() * ACTIONS.length)];
            const a: Action = {
              type,
              start: performance.now(),
              dur: DUR[type],
              tx: 0,
              ty: 0,
              rcx: 0,
              rcy: 0,
              fit: type === "read" ? 1 : fitScale(),
            };
            if (type === "read") readEdgeTarget(a);
            action = a;
          }
          loopAction();
        },
        rand(5500, 12000),
      );

    const frame = (now: number) => {
      raf = requestAnimationFrame(frame);
      if (document.hidden || !wrap || wrap.offsetParent === null) {
        last = now;
        return;
      }
      let dt = (now - last) / 1000;
      last = now;
      if (dt > 0.05) dt = 0.05;

      const botCx = restCx + px;
      const botCy = restCy + py;
      let dx = 0;
      let dy = 0;
      let dist = Infinity;
      if (mousePresent) {
        dx = mouseX - botCx;
        dy = mouseY - botCy;
        dist = Math.hypot(dx, dy) || 0.0001;
      }
      fleeing = mousePresent && dist < FLEE_R;

      let rx = 0;
      let ry = 0;
      let rz = 0;
      let sc = 1;
      let reading = false;

      if (fleeing) {
        action = null;
        const force = 2800 * ((FLEE_R - dist) / FLEE_R);
        vx -= (dx / dist) * force * dt;
        vy -= (dy / dist) * force * dt;
      }

      if (action && !fleeing) {
        if (now - action.start >= action.dur) {
          action = null;
        } else {
          const p = pose(action, now);
          px = p.x * action.fit;
          py = p.y * action.fit;
          rx = p.rx;
          ry = p.ry;
          rz = p.rz;
          sc = p.sc;
          reading = p.reading;
          vx = 0;
          vy = 0;
        }
      }

      if (!action) {
        vx += (-26 * px - 7 * vx) * dt;
        vy += (-26 * py - 7 * vy) * dt;
        px += vx * dt;
        py += vy * dt;
        const r = Math.hypot(px, py);
        if (r > MAX_R) {
          px *= MAX_R / r;
          py *= MAX_R / r;
        }
        rz = clamp(((px - prevPx) / Math.max(dt, 0.001)) * 0.05, -14, 14);
      }

      if (bounds) {
        const b = bounds();
        if (b) {
          const half = size / 2 + 2;
          px = clamp(restCx + px, b.left + half, b.right - half) - restCx;
          py = clamp(restCy + py, b.top + half, b.bottom - half) - restCy;
        }
      }
      prevPx = px;

      let tgx = 0;
      let tgy = 0;
      let thr = 0;
      if (mousePresent && !fleeing && dist < LOOK_R) {
        tgx = (dx / dist) * EYE_MAX;
        tgy = (dy / dist) * EYE_MAX;
        thr = (dx / dist) * HEAD_MAX;
      } else if (reading && action) {
        const ddx = action.rcx - (restCx + px);
        const ddy = action.rcy - (restCy + py);
        const dd = Math.hypot(ddx, ddy) || 1;
        tgx = clamp(ddx / dd + Math.sin(now * 0.006) * 0.5, -1, 1) * 2.2;
        tgy = clamp(ddy / dd, 0, 1) * 2.8;
        thr = clamp(ddx / dd, -1, 1) * HEAD_MAX;
      }
      const gk = 1 - Math.exp(-dt / 0.12);
      gx += (tgx - gx) * gk;
      gy += (tgy - gy) * gk;
      hr += (thr - hr) * gk;
      bs += ((blinkClosed ? 0.12 : 1) - bs) * (1 - Math.exp(-dt / 0.045));
      aAng += (aTarget - aAng) * (1 - Math.exp(-dt / 0.12));

      wrap.style.transform = `translate(${px.toFixed(2)}px, ${py.toFixed(2)}px) perspective(600px) rotateX(${rx.toFixed(2)}deg) rotateY(${ry.toFixed(2)}deg) rotate(${rz.toFixed(2)}deg) scaleY(${sc.toFixed(3)})`;
      if (headEl) headEl.style.transform = `rotate(${hr.toFixed(2)}deg)`;
      if (eyesEl)
        eyesEl.style.transform = `translate(${gx.toFixed(2)}px, ${gy.toFixed(2)}px) scaleY(${bs.toFixed(3)})`;
      if (antennaEl) antennaEl.style.transform = `rotate(${aAng.toFixed(2)}deg)`;
    };

    measure();
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerleave", onLeave);
    window.addEventListener("blur", onLeave);
    window.addEventListener("scroll", measure, { passive: true, capture: true });
    window.addEventListener("resize", measure);
    loopBlink();
    loopAntenna();
    loopAction();
    raf = requestAnimationFrame(frame);

    return () => {
      timers.forEach(clearTimeout);
      cancelAnimationFrame(raf);
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerleave", onLeave);
      window.removeEventListener("blur", onLeave);
      window.removeEventListener("scroll", measure, { capture: true });
      window.removeEventListener("resize", measure);
    };
  });
</script>

<span class="bot-wrap {className}" class:interactive bind:this={wrap}>
  <svg
    class="bot"
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width={strokeWidth}
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <g class="bob">
      <g class="head" bind:this={headEl}>
        <g class="antenna" bind:this={antennaEl}>
          <path d="M12 8V4H8.7" />
        </g>
        <rect width="16" height="12" x="4" y="8" rx="2.5" />
        <path d="M2 14h2" />
        <path d="M20 14h2" />
        <g
          class="eyes"
          bind:this={eyesEl}
          style={interactive ? undefined : `transform: scaleY(${blink ? 0.12 : 1})`}
        >
          <path d="M9 13v2" />
          <path d="M15 13v2" />
        </g>
      </g>
    </g>
  </svg>
</span>

<style>
  .bot-wrap {
    display: inline-block;
    line-height: 0;
    transform-origin: center;
    pointer-events: none;
  }
  .bot {
    display: block;
    overflow: visible;
  }
  .bob {
    animation: bob 3.6s ease-in-out infinite;
    transform-origin: center;
  }
  .head {
    transform-box: fill-box;
    transform-origin: center bottom;
  }
  .antenna {
    transform-box: fill-box;
    transform-origin: right bottom;
  }
  .eyes {
    transform-box: fill-box;
    transform-origin: center;
  }
  .bot-wrap:not(.interactive) .eyes {
    transition: transform 0.09s ease-in-out;
  }
  @keyframes bob {
    0%,
    100% {
      transform: translateY(0) rotate(0deg);
    }
    50% {
      transform: translateY(-2.5px) rotate(0.6deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bob {
      animation: none;
    }
  }
</style>
