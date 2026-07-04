/* =====================================================
   VISUALIZERS — scopes, spectrum, XY, spectrogram, VU,
   curve editor, polar, particle field, piano roll, minimap
   All animated with requestAnimationFrame. Time-driven.
   ===================================================== */

const { useEffect: useVizEffect, useRef: useVizRef, useState: useVizState } = React;

/* ---- shared rAF hook ---- */
function useFrame(cb, deps = []) {
  useVizEffect(() => {
    let raf, t0 = performance.now();
    const tick = (t) => {
      cb((t - t0) / 1000, t);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, deps);
}

/* =====================================================
   OSCILLOSCOPE — phosphor green/cyan waveform
   ===================================================== */
function Scope({ width = 240, height = 80, channel = "cool", freq = 2, amp = 0.6, label = "WAVE" }) {
  const ref = useVizRef(null);
  useFrame((t) => {
    const c = ref.current;
    if (!c) return;
    const ctx = c.getContext("2d");
    const w = c.width, h = c.height;
    ctx.clearRect(0, 0, w, h);

    // grid
    ctx.strokeStyle = "rgba(255,255,255,0.04)";
    ctx.lineWidth = 1;
    for (let i = 1; i < 6; i++) {
      ctx.beginPath();
      ctx.moveTo((w/6)*i, 0); ctx.lineTo((w/6)*i, h);
      ctx.stroke();
    }
    for (let i = 1; i < 4; i++) {
      ctx.beginPath();
      ctx.moveTo(0, (h/4)*i); ctx.lineTo(w, (h/4)*i);
      ctx.stroke();
    }
    // center axis
    ctx.strokeStyle = "rgba(255,255,255,0.08)";
    ctx.beginPath();
    ctx.moveTo(0, h/2); ctx.lineTo(w, h/2);
    ctx.stroke();

    // waveform
    const accent = getComputedStyle(c).getPropertyValue(`--signal-${channel}`).trim() || "#5cf";
    ctx.shadowColor = accent;
    ctx.shadowBlur = 6;
    ctx.strokeStyle = accent;
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    for (let x = 0; x < w; x += 1) {
      const u = x / w;
      // composite signal: fundamental + harmonic + slow drift + noise
      const y = h/2 + Math.sin(u * Math.PI * 2 * freq + t * 2) * h * amp * 0.45
              + Math.sin(u * Math.PI * 2 * freq * 3 + t * 1.4) * h * 0.08
              + (Math.random() - 0.5) * 1.5;
      if (x === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
    }
    ctx.stroke();
    ctx.shadowBlur = 0;
  }, [freq, amp]);

  return (
    <div className="viz mat-screen" style={{ width, height }}>
      <canvas ref={ref} width={width * 2} height={height * 2}
              style={{ width: "100%", height: "100%" }} />
      {label && <div className="viz-label engrave">{label}</div>}
      <div className="viz-corner viz-corner-tl">REC</div>
      <div className="viz-corner viz-corner-tr"><span className="led-dot" /></div>
    </div>
  );
}

/* =====================================================
   SPECTRUM ANALYZER — vertical bars
   ===================================================== */
function Spectrum({ width = 240, height = 90, bands = 32, channel = "life", label = "FFT" }) {
  const ref = useVizRef(null);
  const peaksRef = useVizRef(Array.from({ length: bands }, () => 0));

  useFrame((t) => {
    const c = ref.current; if (!c) return;
    const ctx = c.getContext("2d");
    const w = c.width, h = c.height;
    ctx.clearRect(0, 0, w, h);
    const barW = (w - bands * 2) / bands;

    const accent = getComputedStyle(c).getPropertyValue(`--signal-${channel}`).trim() || "#0e0";

    for (let i = 0; i < bands; i++) {
      // pink-ish noise envelope, decreasing with freq
      const f = i / bands;
      const env = Math.pow(1 - f, 0.6);
      const wob = (Math.sin(t * 3 + i * 0.4) * 0.5 + 0.5) * env;
      const noise = Math.random() * 0.3 * env;
      const v = clamp(wob * 0.7 + noise + Math.sin(t*0.6 + i*0.1)*0.1, 0, 1);

      // peak hold
      peaksRef.current[i] = Math.max(peaksRef.current[i] * 0.97, v);

      const bh = v * h;
      const x = i * (barW + 2);

      // bar
      const grad = ctx.createLinearGradient(0, h - bh, 0, h);
      grad.addColorStop(0, accent);
      grad.addColorStop(1, `${accent}30`);
      ctx.fillStyle = grad;
      ctx.fillRect(x, h - bh, barW, bh);

      // peak hold dot
      const py = h - peaksRef.current[i] * h;
      ctx.fillStyle = accent;
      ctx.shadowColor = accent;
      ctx.shadowBlur = 4;
      ctx.fillRect(x, py - 1, barW, 2);
      ctx.shadowBlur = 0;
    }
  }, []);

  return (
    <div className="viz mat-screen" style={{ width, height }}>
      <canvas ref={ref} width={width*2} height={height*2}
              style={{ width: "100%", height: "100%" }}/>
      {label && <div className="viz-label engrave">{label}</div>}
    </div>
  );
}

/* =====================================================
   SPECTROGRAM — heatmap scrolling left
   ===================================================== */
function Spectrogram({ width = 240, height = 90, label = "SGRAM" }) {
  const ref = useVizRef(null);
  useVizEffect(() => {
    const c = ref.current; if (!c) return;
    const ctx = c.getContext("2d");
    const w = c.width, h = c.height;
    ctx.fillStyle = "#050810"; ctx.fillRect(0, 0, w, h);
    let raf, t = 0;
    const tick = () => {
      // shift left by 1 pixel
      const img = ctx.getImageData(2, 0, w - 2, h);
      ctx.putImageData(img, 0, 0);
      // draw new column on right
      for (let y = 0; y < h; y++) {
        const f = 1 - y / h;
        const v = (Math.sin(t * 0.05 + f * 8) * 0.5 + 0.5) * Math.pow(f, 0.7)
                + Math.random() * 0.2 * f;
        const a = clamp(v, 0, 1);
        // color: violet -> magenta -> orange
        const r = Math.round(40 + a * 215);
        const g = Math.round(20 + a * a * 100);
        const b = Math.round(80 + (1 - a) * 175);
        ctx.fillStyle = `rgb(${r},${g},${b})`;
        ctx.fillRect(w - 2, y, 2, 1);
      }
      t++;
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);
  return (
    <div className="viz mat-screen" style={{ width, height }}>
      <canvas ref={ref} width={width*2} height={height*2}
              style={{ width: "100%", height: "100%" }}/>
      {label && <div className="viz-label engrave">{label}</div>}
    </div>
  );
}

/* =====================================================
   VU METER — horizontal bar with green/yellow/red zones
   ===================================================== */
function VU({ width = 180, height = 14, channel = "life", label = "L" }) {
  const [v, setV] = useVizState(0);
  useFrame((t) => {
    setV(clamp(0.4 + Math.sin(t * 4) * 0.25 + Math.sin(t * 11) * 0.15 + Math.random() * 0.1, 0, 1));
  }, []);
  const segs = 24;
  return (
    <div className="vu" style={{ width, height }}>
      {label && <span className="vu-label engrave">{label}</span>}
      <div className="vu-bar">
        {Array.from({ length: segs }).map((_, i) => {
          const f = i / segs;
          const on = v > f;
          const color = f < 0.6 ? "var(--signal-life)" : f < 0.85 ? "var(--signal-amber)" : "var(--signal-hot)";
          return (
            <div key={i} className="vu-seg" style={{
              background: on ? color : "rgba(255,255,255,0.04)",
              boxShadow: on ? `0 0 4px ${color}` : "none",
            }}/>
          );
        })}
      </div>
    </div>
  );
}

/* =====================================================
   PHASE / XY SCOPE — Lissajous figure
   ===================================================== */
function PhaseScope({ size = 140, channel = "rose", label = "XY" }) {
  const ref = useVizRef(null);
  useFrame((t) => {
    const c = ref.current; if (!c) return;
    const ctx = c.getContext("2d");
    const w = c.width, h = c.height;
    // fade trail
    ctx.fillStyle = "rgba(5,8,16,0.18)";
    ctx.fillRect(0, 0, w, h);
    const accent = getComputedStyle(c).getPropertyValue(`--signal-${channel}`).trim() || "#f48";
    ctx.strokeStyle = accent;
    ctx.shadowColor = accent;
    ctx.shadowBlur = 8;
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    const N = 200;
    for (let i = 0; i < N; i++) {
      const u = i / N;
      const a = u * Math.PI * 2 + t * 0.5;
      const x = w/2 + Math.sin(a * 3 + t) * w * 0.35;
      const y = h/2 + Math.sin(a * 2 + t * 1.3) * h * 0.35;
      if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
    }
    ctx.stroke();
    ctx.shadowBlur = 0;
    // grid cross
    ctx.strokeStyle = "rgba(255,255,255,0.06)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(w/2, 0); ctx.lineTo(w/2, h);
    ctx.moveTo(0, h/2); ctx.lineTo(w, h/2);
    ctx.stroke();
  }, []);
  return (
    <div className="viz mat-screen" style={{ width: size, height: size }}>
      <canvas ref={ref} width={size*2} height={size*2}
              style={{ width: "100%", height: "100%" }}/>
      {label && <div className="viz-label engrave">{label}</div>}
    </div>
  );
}

/* =====================================================
   CURVE EDITOR — envelope / LFO curve with control points
   ===================================================== */
function CurveEditor({ width = 280, height = 100, channel = "myth", label = "ENV", points: initialPoints }) {
  const [points, setPoints] = useVizState(initialPoints || [
    { x: 0, y: 1 }, { x: 0.18, y: 0.4 }, { x: 0.5, y: 0.65 }, { x: 0.82, y: 0.2 }, { x: 1, y: 0 }
  ]);
  const [dragIdx, setDragIdx] = useVizState(-1);
  const ref = useVizRef(null);

  const onDown = (i) => (e) => {
    e.stopPropagation();
    setDragIdx(i);
    const move = (ev) => {
      const r = ref.current.getBoundingClientRect();
      const x = clamp((ev.clientX - r.left) / r.width, 0, 1);
      const y = clamp(1 - (ev.clientY - r.top) / r.height, 0, 1);
      setPoints(prev => prev.map((p, idx) => idx === i ? { ...p, x: i === 0 || i === prev.length - 1 ? p.x : x, y } : p));
    };
    const up = () => {
      setDragIdx(-1);
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  // build smooth path with Catmull-Rom-ish curve
  const path = useMemo(() => {
    const sorted = [...points].sort((a,b) => a.x - b.x);
    const pts = sorted.map(p => ({ x: p.x * width, y: (1 - p.y) * height }));
    let d = `M ${pts[0].x} ${pts[0].y}`;
    for (let i = 0; i < pts.length - 1; i++) {
      const p0 = pts[Math.max(0, i-1)];
      const p1 = pts[i];
      const p2 = pts[i+1];
      const p3 = pts[Math.min(pts.length - 1, i+2)];
      const cp1x = p1.x + (p2.x - p0.x) / 6;
      const cp1y = p1.y + (p2.y - p0.y) / 6;
      const cp2x = p2.x - (p3.x - p1.x) / 6;
      const cp2y = p2.y - (p3.y - p1.y) / 6;
      d += ` C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${p2.x} ${p2.y}`;
    }
    return d;
  }, [points, width, height]);

  const accent = `var(--signal-${channel})`;

  return (
    <div className="viz mat-screen curve-editor" style={{ width, height, "--curve-accent": accent }} ref={ref}>
      <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`}>
        {/* grid */}
        <defs>
          <pattern id="cgrid" width={width/8} height={height/4} patternUnits="userSpaceOnUse">
            <path d={`M 0 0 L 0 ${height/4}`} stroke="rgba(255,255,255,0.05)" />
            <path d={`M 0 0 L ${width/8} 0`} stroke="rgba(255,255,255,0.05)" />
          </pattern>
          <linearGradient id="cfill" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={accent} stopOpacity="0.4"/>
            <stop offset="100%" stopColor={accent} stopOpacity="0"/>
          </linearGradient>
        </defs>
        <rect width={width} height={height} fill="url(#cgrid)" />
        {/* fill area */}
        <path d={`${path} L ${width} ${height} L 0 ${height} Z`} fill="url(#cfill)" />
        {/* curve */}
        <path d={path} stroke={accent} strokeWidth="1.8" fill="none"
              style={{ filter: `drop-shadow(0 0 4px ${accent})` }}/>
        {/* control points */}
        {points.map((p, i) => (
          <g key={i} onPointerDown={onDown(i)} style={{ cursor: "grab" }}>
            <circle cx={p.x * width} cy={(1-p.y) * height} r={dragIdx === i ? 7 : 5}
                    fill={accent} stroke="#fff" strokeWidth="1.2"
                    style={{ filter: `drop-shadow(0 0 6px ${accent})` }}/>
          </g>
        ))}
      </svg>
      {label && <div className="viz-label engrave">{label}</div>}
    </div>
  );
}

/* =====================================================
   POLAR / RADAR PLOT
   ===================================================== */
function Polar({ size = 140, channel = "myth", label = "RADIAL" }) {
  const ref = useVizRef(null);
  useFrame((t) => {
    const c = ref.current; if (!c) return;
    const ctx = c.getContext("2d");
    const w = c.width, h = c.height;
    ctx.clearRect(0, 0, w, h);
    const cx = w/2, cy = h/2, R = w/2 - 8;
    // rings
    ctx.strokeStyle = "rgba(255,255,255,0.06)";
    for (let i = 1; i <= 4; i++) {
      ctx.beginPath();
      ctx.arc(cx, cy, (R/4)*i, 0, Math.PI*2);
      ctx.stroke();
    }
    // axes
    for (let i = 0; i < 12; i++) {
      const a = (i / 12) * Math.PI * 2;
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.lineTo(cx + Math.cos(a) * R, cy + Math.sin(a) * R);
      ctx.stroke();
    }
    // sweep arm
    const sweepA = t * 1.2;
    const grad = ctx.createConicGradient(sweepA - Math.PI*0.2, cx, cy);
    const accent = getComputedStyle(c).getPropertyValue(`--signal-${channel}`).trim() || "#b070ff";
    grad.addColorStop(0, "transparent");
    grad.addColorStop(0.15, accent + "60");
    grad.addColorStop(0.2, accent);
    grad.addColorStop(0.21, "transparent");
    grad.addColorStop(1, "transparent");
    ctx.fillStyle = grad;
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, Math.PI*2); ctx.fill();

    // signal blobs
    ctx.fillStyle = accent;
    ctx.shadowColor = accent;
    ctx.shadowBlur = 8;
    for (let i = 0; i < 6; i++) {
      const a = i * 1.2 + Math.sin(t * 0.3 + i) * 0.3;
      const r = R * (0.3 + (Math.sin(t + i) * 0.5 + 0.5) * 0.5);
      const x = cx + Math.cos(a) * r;
      const y = cy + Math.sin(a) * r;
      ctx.beginPath(); ctx.arc(x, y, 2.5, 0, Math.PI*2); ctx.fill();
    }
    ctx.shadowBlur = 0;
  }, []);
  return (
    <div className="viz mat-screen" style={{ width: size, height: size }}>
      <canvas ref={ref} width={size*2} height={size*2}
              style={{ width: "100%", height: "100%" }}/>
      {label && <div className="viz-label engrave">{label}</div>}
    </div>
  );
}

/* =====================================================
   PARTICLE FIELD — flow visualization
   ===================================================== */
function ParticleField({ width = 280, height = 120, channel = "cool", label = "FLOW", count = 80 }) {
  const ref = useVizRef(null);
  const particlesRef = useVizRef(null);

  useVizEffect(() => {
    particlesRef.current = Array.from({ length: count }, () => ({
      x: Math.random() * width,
      y: Math.random() * height,
      vx: 0, vy: 0,
      life: Math.random(),
    }));
  }, [count, width, height]);

  useFrame((t) => {
    const c = ref.current; if (!c) return;
    const ctx = c.getContext("2d");
    const w = c.width, h = c.height;
    ctx.fillStyle = "rgba(5,8,16,0.2)";
    ctx.fillRect(0, 0, w, h);
    const accent = getComputedStyle(c).getPropertyValue(`--signal-${channel}`).trim() || "#5cf";

    const ps = particlesRef.current;
    if (!ps) return;
    for (const p of ps) {
      // curl noise field
      const fx = (p.x / width) * 4;
      const fy = (p.y / height) * 3;
      const ang = Math.sin(fx + t * 0.4) + Math.cos(fy + t * 0.3) + Math.sin(fx*2 - fy + t*0.2);
      p.vx = p.vx * 0.92 + Math.cos(ang) * 0.6;
      p.vy = p.vy * 0.92 + Math.sin(ang) * 0.6;
      p.x += p.vx; p.y += p.vy;
      p.life -= 0.005;
      if (p.x < 0 || p.x > width || p.y < 0 || p.y > height || p.life <= 0) {
        p.x = Math.random() * width;
        p.y = Math.random() * height;
        p.life = 1;
      }
      const alpha = p.life * 0.9;
      ctx.fillStyle = accent;
      ctx.globalAlpha = alpha;
      ctx.shadowColor = accent;
      ctx.shadowBlur = 6;
      ctx.beginPath();
      ctx.arc(p.x * 2, p.y * 2, 1.4, 0, Math.PI*2);
      ctx.fill();
    }
    ctx.globalAlpha = 1;
    ctx.shadowBlur = 0;
  }, []);

  return (
    <div className="viz mat-screen" style={{ width, height }}>
      <canvas ref={ref} width={width*2} height={height*2}
              style={{ width: "100%", height: "100%" }}/>
      {label && <div className="viz-label engrave">{label}</div>}
    </div>
  );
}

/* =====================================================
   PIANO ROLL STRIP
   ===================================================== */
function PianoRoll({ width = 280, height = 80, channel = "myth", label = "ROLL" }) {
  const notes = useMemo(() => {
    const out = [];
    for (let i = 0; i < 28; i++) {
      out.push({
        x: Math.random() * 0.95,
        y: Math.floor(Math.random() * 12),
        len: 0.04 + Math.random() * 0.12,
        v: 0.3 + Math.random() * 0.7,
      });
    }
    return out;
  }, []);
  const accent = `var(--signal-${channel})`;
  const [playhead, setPlayhead] = useVizState(0);
  useFrame((t) => setPlayhead((t * 0.18) % 1), []);
  return (
    <div className="viz mat-screen" style={{ width, height }}>
      <svg width={width} height={height}>
        {/* horizontal lines for pitches */}
        {Array.from({ length: 12 }).map((_, i) => (
          <line key={i} x1="0" x2={width}
                y1={(i + 0.5) * (height / 12)} y2={(i + 0.5) * (height / 12)}
                stroke="rgba(255,255,255,0.04)" />
        ))}
        {/* notes */}
        {notes.map((n, i) => (
          <rect key={i}
            x={n.x * width}
            y={n.y * (height / 12) + 1}
            width={n.len * width}
            height={(height / 12) - 2}
            rx="1.5"
            fill={accent}
            opacity={n.v}
            style={{ filter: `drop-shadow(0 0 3px ${accent})` }}/>
        ))}
        {/* playhead */}
        <line x1={playhead * width} x2={playhead * width} y1="0" y2={height}
              stroke={accent} strokeWidth="1.5" opacity="0.8"
              style={{ filter: `drop-shadow(0 0 4px ${accent})` }}/>
      </svg>
      {label && <div className="viz-label engrave">{label}</div>}
    </div>
  );
}

/* =====================================================
   WAVEFORM (filled / static-looking — like an audio file preview)
   ===================================================== */
function Waveform({ width = 280, height = 60, channel = "rose", label = "CLIP" }) {
  const peaks = useMemo(() => Array.from({ length: 80 }, (_, i) => {
    const env = Math.sin((i / 80) * Math.PI);
    return env * (0.4 + Math.random() * 0.6);
  }), []);
  const accent = `var(--signal-${channel})`;
  return (
    <div className="viz mat-screen" style={{ width, height }}>
      <svg width={width} height={height}>
        {peaks.map((p, i) => {
          const x = (i / peaks.length) * width;
          const w = (width / peaks.length) - 1;
          const h = p * height * 0.85;
          return (
            <rect key={i} x={x} y={(height - h)/2} width={Math.max(1,w)} height={h}
                  fill={accent} opacity={0.85}
                  style={{ filter: `drop-shadow(0 0 2px ${accent})` }}/>
          );
        })}
      </svg>
      {label && <div className="viz-label engrave">{label}</div>}
    </div>
  );
}

/* =====================================================
   MINIMAP — node graph overview
   ===================================================== */
function NodeMinimap({ width = 160, height = 90, label = "MAP" }) {
  return (
    <div className="viz mat-screen" style={{ width, height }}>
      <svg width={width} height={height}>
        {/* fake node rectangles */}
        {[
          { x: 10, y: 18, c: "var(--ch01-atlas)" },
          { x: 36, y: 32, c: "var(--ch14-continuum)" },
          { x: 62, y: 12, c: "var(--ch07-instinct)" },
          { x: 64, y: 50, c: "var(--ch12-composer)" },
          { x: 96, y: 28, c: "var(--ch10-quill)" },
          { x: 122, y: 46, c: "var(--ch16-nexus)" },
          { x: 28, y: 64, c: "var(--ch15-forge)" },
          { x: 96, y: 64, c: "var(--ch11-codex)" },
        ].map((n,i)=>(
          <rect key={i} x={n.x} y={n.y} width="14" height="6" rx="1" fill={n.c} opacity="0.85"
                style={{ filter: `drop-shadow(0 0 2px ${n.c})` }}/>
        ))}
        {/* connectors */}
        <g stroke="rgba(120,180,255,0.35)" strokeWidth="0.8" fill="none">
          <path d="M 24 21 C 30 21, 30 35, 36 35" />
          <path d="M 50 35 C 56 35, 56 15, 62 15" />
          <path d="M 76 15 C 84 15, 84 31, 96 31" />
          <path d="M 78 53 C 88 53, 88 67, 96 67" />
          <path d="M 110 31 C 116 31, 116 49, 122 49" />
        </g>
        {/* viewport rect */}
        <rect x="58" y="20" width="50" height="36" fill="none" stroke="var(--signal-cool)"
              strokeWidth="1" strokeDasharray="3 2" opacity="0.7"/>
      </svg>
      {label && <div className="viz-label engrave">{label}</div>}
    </div>
  );
}

/* expose */
Object.assign(window, {
  Scope, Spectrum, Spectrogram, VU, PhaseScope,
  CurveEditor, Polar, ParticleField, PianoRoll, Waveform, NodeMinimap,
  useFrame,
});
