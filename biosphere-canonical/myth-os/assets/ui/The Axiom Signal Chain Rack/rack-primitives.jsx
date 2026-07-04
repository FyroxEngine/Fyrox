/* global React */
// ═══════════════════════════════════════════════════════════════════
// RACK PRIMITIVES — interactive controls for the Axiom Signal Chain Rack
// Knob · Fader · Jack · SeqStep · Lissajous · Aurora · Jog · ResonanceMeter
// ═══════════════════════════════════════════════════════════════════

const { useState, useRef, useEffect, useMemo, useCallback, useContext, createContext } = React;

// ── SignalContext: shared rack state ─────────────────────────────────
const SignalContext = createContext(null);
const useSignal = () => useContext(SignalContext);

// ── Glyph icons (small inline SVGs) ──────────────────────────────────
const GlyphSigil = ({ size = 18, color = 'currentColor' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="1.2">
    <circle cx="12" cy="12" r="9"/>
    <path d="M12 3v6M12 15v6M3 12h6M15 12h6"/>
    <circle cx="12" cy="12" r="3"/>
  </svg>
);

const GlyphHerald = ({ size = 18, color = 'currentColor' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="1.2">
    <path d="M12 2 L20 6 V12 C20 17 16 21 12 22 C8 21 4 17 4 12 V6 Z"/>
    <path d="M12 8 L12 16 M8 12 L16 12"/>
  </svg>
);

// ── KNOB ─────────────────────────────────────────────────────────────
// Drag vertically to turn. Range -135° to +135° → maps to 0..1.
function Knob({ label, sublabel, value, onChange, size = 'md', valueFormatter, channel = 'q', sigil, compact = false }) {
  const ref = useRef(null);
  const [dragging, setDragging] = useState(false);
  const startY = useRef(0);
  const startVal = useRef(value);

  const onPointerDown = (e) => {
    e.preventDefault();
    setDragging(true);
    startY.current = e.clientY;
    startVal.current = value;
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e) => {
    if (!dragging) return;
    const dy = startY.current - e.clientY; // up = positive
    const factor = e.shiftKey ? 0.0015 : 0.005;
    const next = Math.max(0, Math.min(1, startVal.current + dy * factor));
    onChange(next);
  };
  const onPointerUp = (e) => {
    setDragging(false);
    try { e.currentTarget.releasePointerCapture(e.pointerId); } catch {}
  };
  const onDoubleClick = () => onChange(0.5);

  // Map 0..1 → -135..+135°
  const angle = -135 + value * 270;
  const sizeClass = size === 'lg' ? 'lg' : size === 'sm' ? 'sm' : size === 'xs' ? 'xs' : '';

  const display = valueFormatter
    ? valueFormatter(value)
    : Math.round(value * 100).toString().padStart(3, '0');

  // Tick arc
  const radius = 38;
  const ticks = [];
  for (let i = 0; i <= 12; i++) {
    const a = -135 + (i / 12) * 270;
    const rad = (a - 90) * Math.PI / 180;
    const x1 = 50 + Math.cos(rad) * radius;
    const y1 = 50 + Math.sin(rad) * radius;
    const x2 = 50 + Math.cos(rad) * (radius - 4);
    const y2 = 50 + Math.sin(rad) * (radius - 4);
    const lit = (i / 12) <= value;
    ticks.push(<line key={i} x1={x1} y1={y1} x2={x2} y2={y2} stroke={lit ? `var(--d-color)` : 'rgba(120,180,255,0.18)'} strokeWidth={lit ? 1.5 : 1} />);
  }

  return (
    <div className="knob-wrap" data-channel={channel}>
      {label && !compact && <div className="knob-label">{label}</div>}
      <div
        ref={ref}
        className={`knob ${sizeClass} ${dragging ? 'dragging' : ''}`}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onDoubleClick={onDoubleClick}
        style={{
          '--d-color': `var(--${channel === 'q' ? 'astral-cyan' : channel === 'm' ? 'astral-magenta' : channel === 'b' ? 'bio' : channel === 'g' ? 'gold' : 'astral-cyan'})`,
          '--d-glow':  `var(--${channel === 'q' ? 'astral-cyan-glow' : channel === 'm' ? 'astral-magenta-glow' : channel === 'b' ? 'bio-glow' : channel === 'g' ? 'gold-glow' : 'astral-cyan-glow'})`,
        }}
      >
        <svg className="tick-arc" viewBox="0 0 100 100" style={{ position: 'absolute', inset: -6, width: 'calc(100% + 12px)', height: 'calc(100% + 12px)' }}>
          {ticks}
        </svg>
        <div className="indicator" style={{ transform: `rotate(${angle}deg)` }} />
      </div>
      <div className="knob-value">{display}</div>
      {sublabel && !compact && <div className="knob-sublabel">{sublabel}</div>}
    </div>
  );
}

// ── FADER ────────────────────────────────────────────────────────────
function Fader({ label, archetype, value, onChange, channel = 'q', height = 180 }) {
  const trackRef = useRef(null);
  const [dragging, setDragging] = useState(false);

  const setFromY = (clientY) => {
    const r = trackRef.current.getBoundingClientRect();
    const pct = 1 - Math.max(0, Math.min(1, (clientY - r.top) / r.height));
    onChange(pct);
  };

  const onPointerDown = (e) => {
    e.preventDefault();
    setDragging(true);
    setFromY(e.clientY);
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e) => { if (dragging) setFromY(e.clientY); };
  const onPointerUp = (e) => { setDragging(false); try { e.currentTarget.releasePointerCapture(e.pointerId); } catch {} };

  const ch = channel === 'q' ? 'astral-cyan' : channel === 'm' ? 'astral-magenta' : channel === 'b' ? 'bio' : channel === 'g' ? 'gold' : 'astral-cyan';
  const chGlow = ch + '-glow';

  return (
    <div className="fader-wrap" style={{ '--d-color': `var(--${ch})`, '--d-glow': `var(--${chGlow})` }}>
      <div className="fader-label">{label}</div>
      <div
        ref={trackRef}
        className="fader-track"
        style={{ height }}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
      >
        <div className="scale" />
        <div className="lit" style={{ height: `${value * 100}%` }} />
        <div className="fader-thumb" style={{ bottom: `calc(${value * 100}% - 9px)` }}>
          <div className="led" />
        </div>
      </div>
      <div className="fader-value">{Math.round(value * 127).toString().padStart(3, '0')}</div>
      {archetype && <div className="fader-archetype">{archetype}</div>}
    </div>
  );
}

// ── JACK ─────────────────────────────────────────────────────────────
function Jack({ id, kind = 'out', label, channel }) {
  const { armedJack, setArmedJack, cables, addCable, removeCableAt } = useSignal();
  const ref = useRef(null);

  useEffect(() => {
    if (!ref.current) return;
    // Register jack position for cable drawing — uses a global registry
    const el = ref.current;
    window.__jackRegistry = window.__jackRegistry || new Map();
    window.__jackRegistry.set(id, el);
    return () => { window.__jackRegistry.delete(id); };
  }, [id]);

  const isPatched = cables.some(c => c.from === id || c.to === id);
  const armed = armedJack && armedJack.id === id;

  const onClick = (e) => {
    e.stopPropagation();
    if (isPatched) {
      // Disconnect
      removeCableAt(id);
      return;
    }
    if (!armedJack) {
      setArmedJack({ id, kind });
      return;
    }
    if (armedJack.id === id) {
      setArmedJack(null);
      return;
    }
    // Need opposite kinds
    if (armedJack.kind === kind) {
      // re-arm with this one
      setArmedJack({ id, kind });
      return;
    }
    const from = armedJack.kind === 'out' ? armedJack.id : id;
    const to   = armedJack.kind === 'in'  ? armedJack.id : id;
    addCable({ from, to });
    setArmedJack(null);
  };

  const ch = kind === 'in' ? 'in' : kind === 'cv' ? 'cv' : 'out';

  return (
    <div className="jack-row">
      <div
        ref={ref}
        className={`jack ${ch} ${armed ? 'armed' : ''} ${isPatched ? 'patched' : ''}`}
        data-jack-id={id}
        onClick={onClick}
        title={`${label || id} (${kind})`}
      />
      {label && <div className="lbl">{label}</div>}
    </div>
  );
}

// ── SEQ STEP ─────────────────────────────────────────────────────────
function SeqStep({ idx, on, playing, onToggle }) {
  return (
    <div
      className={`seq-step ${on ? 'on' : ''} ${playing ? 'playing' : ''}`}
      onClick={() => onToggle(idx)}
    >
      <span className="glyph">{(idx + 1).toString().padStart(2, '0')}</span>
    </div>
  );
}

// ── LISSAJOUS ────────────────────────────────────────────────────────
function Lissajous({ a, b, delta, size = 200, color = 'var(--astral-cyan)', secondary = 'var(--astral-magenta)' }) {
  const [t, setT] = useState(0);
  useEffect(() => {
    let raf;
    const tick = () => { setT(performance.now() / 1000); raf = requestAnimationFrame(tick); };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);

  // Generate path from sin/cos
  const aMul = 1 + a * 6;
  const bMul = 1 + b * 6;
  const phase = delta * Math.PI * 2 + t * 0.3;
  const r = 0.42;
  const pts = [];
  const N = 240;
  for (let i = 0; i <= N; i++) {
    const u = (i / N) * Math.PI * 2;
    const x = 0.5 + r * Math.sin(aMul * u + phase);
    const y = 0.5 + r * Math.sin(bMul * u);
    pts.push(`${(x * size).toFixed(2)},${(y * size).toFixed(2)}`);
  }

  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} style={{ position: 'absolute', inset: 0 }}>
      <defs>
        <radialGradient id="liss-grad" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor={color} stopOpacity="0.7" />
          <stop offset="60%" stopColor={secondary} stopOpacity="0.6" />
          <stop offset="100%" stopColor={color} stopOpacity="0.3" />
        </radialGradient>
        <filter id="liss-glow">
          <feGaussianBlur stdDeviation="2.2" result="b"/>
          <feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge>
        </filter>
      </defs>
      <polyline points={pts.join(' ')} fill="none" stroke="url(#liss-grad)" strokeWidth="1.4" filter="url(#liss-glow)" opacity="0.95"/>
      <polyline points={pts.join(' ')} fill="none" stroke={color} strokeWidth="0.6" opacity="0.6"/>
    </svg>
  );
}

// ── AURORA (Persona spectrum) ────────────────────────────────────────
function Aurora({ values }) {
  const [t, setT] = useState(0);
  useEffect(() => {
    let raf;
    const tick = () => { setT(performance.now() / 1000); raf = requestAnimationFrame(tick); };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);

  const W = 600, H = 140;
  const layers = ['hero', 'trickster', 'sage', 'shadow', 'anima'];
  const colors = ['#00bfff', '#ff1493', '#fbbf24', '#9400d3', '#39ff14'];

  return (
    <svg width="100%" height={H} viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" style={{ display: 'block' }}>
      <defs>
        {colors.map((c, i) => (
          <linearGradient key={i} id={`aur${i}`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={c} stopOpacity="0" />
            <stop offset="50%" stopColor={c} stopOpacity={0.4 + (values[i] || 0) * 0.5} />
            <stop offset="100%" stopColor={c} stopOpacity="0" />
          </linearGradient>
        ))}
      </defs>
      {layers.map((_, i) => {
        const v = values[i] || 0;
        const amp = 30 + v * 50;
        const freq = 1.4 + i * 0.5;
        const phase = t * (0.4 + i * 0.15);
        const pts = [];
        for (let x = 0; x <= W; x += 8) {
          const u = x / W;
          const y = H / 2 + Math.sin(u * Math.PI * freq + phase) * amp + (i - 2) * 10;
          pts.push(`${x},${y}`);
        }
        const top = pts.map(p => p).join(' ');
        const bot = pts.map(p => { const [x, y] = p.split(','); return `${x},${parseFloat(y) + 35}`; }).reverse().join(' ');
        return (
          <polygon
            key={i}
            points={`${top} ${bot}`}
            fill={`url(#aur${i})`}
            opacity={0.55 + v * 0.4}
            style={{ mixBlendMode: 'screen' }}
          />
        );
      })}
      {/* Reference grid */}
      <line x1="0" y1={H/2} x2={W} y2={H/2} stroke="rgba(255,255,255,0.05)" strokeDasharray="2 4" />
    </svg>
  );
}

// ── JOG / EPOCH SCRUBBER ─────────────────────────────────────────────
function JogWheel({ value, onChange, epoch }) {
  const ref = useRef(null);
  const [dragging, setDragging] = useState(false);
  const last = useRef({ x: 0, y: 0 });

  const center = useRef({ x: 0, y: 0 });

  const angleOf = (cx, cy, x, y) => Math.atan2(y - cy, x - cx);

  const onPointerDown = (e) => {
    e.preventDefault();
    setDragging(true);
    const r = ref.current.getBoundingClientRect();
    center.current = { x: r.left + r.width / 2, y: r.top + r.height / 2 };
    last.current = { angle: angleOf(center.current.x, center.current.y, e.clientX, e.clientY) };
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e) => {
    if (!dragging) return;
    const a = angleOf(center.current.x, center.current.y, e.clientX, e.clientY);
    let d = a - last.current.angle;
    if (d > Math.PI) d -= 2 * Math.PI;
    if (d < -Math.PI) d += 2 * Math.PI;
    last.current.angle = a;
    onChange(value + d / (Math.PI * 2));
  };
  const onPointerUp = (e) => { setDragging(false); try { e.currentTarget.releasePointerCapture(e.pointerId); } catch {} };

  const rotation = value * 360;

  return (
    <div
      ref={ref}
      className="jog"
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
    >
      <div className="outer-ring" style={{ transform: `rotate(${rotation}deg)` }} />
      <div className="ticks" />
      <div className="inner">
        <div className="epoch">EPOCH</div>
        <div className="num">{(((value % 1) + 1) % 1 * 1000).toFixed(0).padStart(3, '0')}</div>
        <div style={{ fontFamily: 'var(--font-code)', fontSize: 7.5, letterSpacing: '0.2em', color: 'var(--fg-muted)', textTransform: 'uppercase', marginTop: 2 }}>
          {epoch || 'PRESENT'}
        </div>
      </div>
    </div>
  );
}

// ── RESONANCE METER ──────────────────────────────────────────────────
function ResonanceMeter({ value, harmonics }) {
  const pct = Math.max(0, Math.min(1, value));
  const status = pct < 0.25 ? 'DISSONANT' : pct < 0.55 ? 'UNSTABLE' : pct < 0.8 ? 'COHERENT' : 'RESONANT';
  const statusColor = pct < 0.25 ? 'var(--ember)' : pct < 0.55 ? 'var(--rose)' : pct < 0.8 ? 'var(--astral-cyan)' : 'var(--bio)';

  return (
    <div className="resonance-meter">
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
        <div style={{ fontFamily: 'var(--font-header)', fontSize: 10, letterSpacing: '0.28em', color: 'var(--fg-2)', textTransform: 'uppercase' }}>SYSTEM RESONANCE</div>
        <div style={{ fontFamily: 'var(--font-code)', fontSize: 9, letterSpacing: '0.18em', color: statusColor, textShadow: `0 0 8px ${statusColor}` }}>
          {status}
        </div>
      </div>
      <div className="resonance-bar">
        <div className="fill" style={{ width: `${pct * 100}%` }} />
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.2em', color: 'var(--fg-muted)', textTransform: 'uppercase' }}>
        <span>φ {(pct * 432).toFixed(1)}hz</span>
        <span>Δ {harmonics?.toFixed(2) ?? '0.00'}</span>
        <span>{(pct * 100).toFixed(0)}%</span>
      </div>
    </div>
  );
}

// ── CONSTELLATION (Nebula Systems) ───────────────────────────────────
function Constellation({ nodes }) {
  return (
    <svg viewBox="0 0 600 160" preserveAspectRatio="none" style={{ width: '100%', height: '100%' }}>
      <defs>
        <filter id="const-glow">
          <feGaussianBlur stdDeviation="2"/>
        </filter>
        <radialGradient id="neb-r" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="#ff1493" stopOpacity="0.4"/>
          <stop offset="100%" stopColor="#ff1493" stopOpacity="0"/>
        </radialGradient>
        <radialGradient id="neb-b" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="#00bfff" stopOpacity="0.35"/>
          <stop offset="100%" stopColor="#00bfff" stopOpacity="0"/>
        </radialGradient>
      </defs>
      {/* nebula clouds */}
      <ellipse cx="120" cy="80" rx="120" ry="60" fill="url(#neb-b)"/>
      <ellipse cx="440" cy="90" rx="110" ry="50" fill="url(#neb-r)"/>
      {/* constellation lines */}
      {nodes.map((cluster, ci) => (
        <g key={ci}>
          {cluster.lines.map((l, li) => (
            <line key={li} x1={cluster.pts[l[0]][0]} y1={cluster.pts[l[0]][1]} x2={cluster.pts[l[1]][0]} y2={cluster.pts[l[1]][1]} stroke={cluster.color} strokeWidth="0.8" strokeOpacity="0.5"/>
          ))}
          {cluster.pts.map((p, pi) => (
            <g key={pi}>
              <circle cx={p[0]} cy={p[1]} r="2.5" fill={cluster.color} filter="url(#const-glow)" opacity="0.9"/>
              <circle cx={p[0]} cy={p[1]} r="1.4" fill="#fff"/>
            </g>
          ))}
        </g>
      ))}
    </svg>
  );
}

// ── LATTICE (Void Lattice Engineer) ──────────────────────────────────
function Lattice({ stress }) {
  // Generate a static-ish grid where some cells are stressed based on `stress` value
  const cells = useMemo(() => {
    const arr = [];
    for (let i = 0; i < 72; i++) {
      // pseudo-random but stable
      const r = ((i * 9301 + 49297) % 233280) / 233280;
      arr.push(r);
    }
    return arr;
  }, []);
  const threshold = 1 - stress * 0.7;

  return (
    <div className="lattice-grid">
      {cells.map((c, i) => {
        const stressed = c > threshold;
        const hot = c > threshold + 0.15;
        const color = hot
          ? `rgba(249,115,22,${0.6 + (c - threshold) * 0.5})`
          : stressed
            ? `rgba(251,191,36,${0.35 + (c - threshold) * 0.6})`
            : `rgba(0,229,255,${0.04 + c * 0.18})`;
        const glow = hot ? '0 0 8px rgba(249,115,22,0.7)' : stressed ? '0 0 6px rgba(251,191,36,0.45)' : '0 0 4px rgba(0,229,255,0.18)';
        return (
          <div
            key={i}
            className="lattice-cell"
            style={{
              background: color,
              borderColor: hot ? 'rgba(249,115,22,0.5)' : stressed ? 'rgba(251,191,36,0.35)' : 'rgba(0,229,255,0.18)',
              boxShadow: glow,
            }}
          />
        );
      })}
    </div>
  );
}

// ── MYTHOS MAP (narrative graph) ─────────────────────────────────────
function MythosMap({ activeArc }) {
  const arcs = [
    { id: 'origin',     pts: [[40, 80], [110, 50], [180, 90], [220, 60]] },
    { id: 'descent',    pts: [[220, 60], [280, 110], [320, 85], [380, 130]] },
    { id: 'apotheosis', pts: [[380, 130], [440, 90], [490, 60], [560, 80]] },
  ];
  const allPts = arcs.flatMap(a => a.pts);
  return (
    <svg viewBox="0 0 600 180" preserveAspectRatio="none" style={{ width: '100%', height: '100%' }}>
      <defs>
        <filter id="myth-glow">
          <feGaussianBlur stdDeviation="2.5"/>
          <feMerge><feMergeNode/><feMergeNode in="SourceGraphic"/></feMerge>
        </filter>
        <linearGradient id="myth-grad" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stopColor="#9400d3"/>
          <stop offset="50%" stopColor="#fbbf24"/>
          <stop offset="100%" stopColor="#ff1493"/>
        </linearGradient>
      </defs>
      {/* background grid */}
      {[1,2,3,4].map(i => (
        <line key={i} x1="0" y1={36 * i} x2="600" y2={36 * i} stroke="rgba(192,132,252,0.06)" strokeDasharray="3 5"/>
      ))}
      {/* arcs */}
      {arcs.map((a, i) => {
        const active = a.id === activeArc;
        const path = a.pts.reduce((s, p, j) => s + (j === 0 ? `M ${p[0]} ${p[1]}` : ` L ${p[0]} ${p[1]}`), '');
        return (
          <g key={a.id}>
            <path d={path} fill="none" stroke={active ? 'url(#myth-grad)' : 'rgba(192,132,252,0.4)'} strokeWidth={active ? 2.2 : 1.4} filter={active ? 'url(#myth-glow)' : undefined}/>
            {a.pts.map((p, pi) => (
              <g key={pi}>
                <circle cx={p[0]} cy={p[1]} r={active ? 4 : 3} fill={active ? '#fbbf24' : '#c084fc'} filter="url(#myth-glow)"/>
                <circle cx={p[0]} cy={p[1]} r="1.5" fill="#fff"/>
              </g>
            ))}
            <text x={a.pts[0][0]} y={a.pts[0][1] - 12} fill="rgba(192,132,252,0.7)" fontFamily="var(--font-code)" fontSize="8" letterSpacing="2">
              {a.id.toUpperCase()}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

// ── HARMONIC METERS (Resonance Sealer) ───────────────────────────────
function HarmonicMeters({ values }) {
  // values: array of 0..1
  return (
    <div style={{ display: 'flex', gap: 4, alignItems: 'flex-end', height: 80, padding: '8px 4px', background: 'var(--panel-screen)', borderRadius: 3, border: '1px solid rgba(0,229,255,0.15)' }}>
      {values.map((v, i) => {
        const peak = v > 0.85;
        const h = Math.max(0.05, v) * 100;
        return (
          <div key={i} style={{ flex: 1, height: '100%', display: 'flex', flexDirection: 'column', justifyContent: 'flex-end', position: 'relative' }}>
            <div style={{
              height: `${h}%`,
              background: peak
                ? 'linear-gradient(180deg, var(--ember), var(--gold), var(--bio))'
                : 'linear-gradient(180deg, var(--rose), var(--mythos), var(--astral-cyan), var(--bio))',
              borderRadius: 1,
              boxShadow: `0 0 ${4 + v * 8}px ${peak ? 'var(--ember-glow)' : 'var(--astral-cyan-glow)'}`,
              transition: 'height 180ms var(--ease-out)',
            }}/>
            <div style={{ position: 'absolute', bottom: -10, left: '50%', transform: 'translateX(-50%)', fontFamily: 'var(--font-code)', fontSize: 6, color: 'var(--fg-muted)', letterSpacing: '0.1em' }}>
              {(i + 1).toString().padStart(2, '0')}
            </div>
          </div>
        );
      })}
    </div>
  );
}

// ── LORE TOOLTIP ─────────────────────────────────────────────────────
function useLore() {
  const [lore, setLore] = useState(null);
  const show = (info, e) => {
    setLore({ ...info, x: e.clientX, y: e.clientY });
  };
  const hide = () => setLore(null);
  const node = lore ? (
    <div className="lore-card" style={{ left: Math.min(window.innerWidth - 340, lore.x + 16), top: lore.y + 16 }}>
      <div className="lore-title">
        {lore.glyph && <span className="lore-glyph">{lore.glyph}</span>}
        {lore.title}
      </div>
      <div className="lore-body">"{lore.body}"</div>
      {lore.tag && <div className="lore-tag">{lore.tag}</div>}
    </div>
  ) : null;
  return { show, hide, node };
}

// ── CABLES LAYER ─────────────────────────────────────────────────────
function CablesLayer({ cables, style }) {
  const [tick, setTick] = useState(0);
  // Rebuild positions on every animation frame to keep cables glued during scroll/resize
  useEffect(() => {
    let raf;
    const loop = () => { setTick(t => t + 1); raf = requestAnimationFrame(loop); };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, []);

  const getCenter = (id) => {
    const el = window.__jackRegistry?.get(id);
    if (!el) return null;
    const r = el.getBoundingClientRect();
    const layer = document.querySelector('.cables-layer');
    if (!layer) return null;
    const lr = layer.getBoundingClientRect();
    return { x: r.left + r.width/2 - lr.left, y: r.top + r.height/2 - lr.top };
  };

  // Per-cable color palette
  const cableColors = ['#00bfff', '#ff1493', '#9400d3', '#39ff14', '#fbbf24', '#fb7185'];

  const fiberish = style === 'fiber';
  const chainish = style === 'chain';

  return (
    <svg className="cables-layer" style={{ width: '100%', height: '100%' }}>
      <defs>
        <filter id="cable-glow">
          <feGaussianBlur stdDeviation={fiberish ? 1.4 : 3} result="b"/>
          <feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge>
        </filter>
      </defs>
      {cables.map((c, i) => {
        const a = getCenter(c.from);
        const b = getCenter(c.to);
        if (!a || !b) return null;
        const sag = Math.min(120, 30 + Math.abs(b.x - a.x) * 0.18);
        const midY = Math.max(a.y, b.y) + sag;
        const midX = (a.x + b.x) / 2;
        const path = `M ${a.x} ${a.y} Q ${midX} ${midY} ${b.x} ${b.y}`;
        const color = cableColors[i % cableColors.length];
        if (chainish) {
          return (
            <g key={i}>
              <path d={path} stroke={color} strokeOpacity="0.5" strokeWidth="3" fill="none" strokeDasharray="2 6" filter="url(#cable-glow)"/>
              <path d={path} stroke={color} strokeWidth="1.5" fill="none" strokeDasharray="2 6"/>
            </g>
          );
        }
        return (
          <g key={i}>
            {!fiberish && <path d={path} stroke={color} strokeOpacity="0.35" strokeWidth="7" fill="none" filter="url(#cable-glow)"/>}
            <path d={path} stroke={color} strokeWidth={fiberish ? 1.2 : 2.2} fill="none" filter="url(#cable-glow)"/>
            {fiberish && <path d={path} stroke="#fff" strokeOpacity="0.6" strokeWidth="0.4" fill="none"/>}
            <circle cx={a.x} cy={a.y} r="3" fill={color} filter="url(#cable-glow)"/>
            <circle cx={b.x} cy={b.y} r="3" fill={color} filter="url(#cable-glow)"/>
          </g>
        );
      })}
    </svg>
  );
}

// ── CHANNEL STRIP ────────────────────────────────────────────────────
// Compact vertical column with stacked knobs + meter + fader. Modeled after
// a mixing-console channel strip. `wire` sets the colour (Genesis wire type).
function ChannelStrip({ name, sub, wire = 'SPA', knobs = [], pads = [], fader, meter = 0.5, jacks }) {
  // wire color
  const wireColor = `var(--wire-${wire})`;
  const wireGlow = `${wireColor}`;
  return (
    <div className="channel-strip" style={{ '--d-color': wireColor, '--d-glow': wireGlow, '--wire': wireColor }}>
      <div className="cs-head">{name}</div>
      {sub && <div className="cs-sub">{sub}</div>}

      {/* Stacked knobs (2 cols when 4+) */}
      <div style={{ display: 'grid', gridTemplateColumns: knobs.length > 2 ? 'repeat(2, 1fr)' : '1fr', gap: 6, paddingTop: 2 }}>
        {knobs.map((k, i) => (
          <div key={i} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 1 }}>
            <Knob
              value={k.value}
              onChange={k.onChange}
              size="sm"
              channel="q"
              compact
              valueFormatter={() => ''}
            />
            <div style={{ fontFamily: 'var(--font-code)', fontSize: 6, letterSpacing: '0.1em', color: 'var(--fg-muted)', textTransform: 'uppercase', textAlign: 'center', maxWidth: 24, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{k.label}</div>
          </div>
        ))}
      </div>

      {/* Pads (mute/solo/arm) */}
      {pads.length > 0 && (
        <div className="cs-row">
          {pads.map((p, i) => (
            <div key={i} className={`cs-pad ${p.on ? 'on' : ''}`} onClick={p.onClick} title={p.title}>{p.label}</div>
          ))}
        </div>
      )}

      {/* Meter + fader side-by-side */}
      <div style={{ display: 'flex', gap: 6, alignItems: 'flex-end', justifyContent: 'center', flex: 1 }}>
        <div className="cs-meter">
          <div className="fill" style={{ height: `${Math.max(2, meter * 100)}%`, color: wireColor }}/>
          <div className="seg-marks"/>
        </div>
        {fader && (
          <Fader
            value={fader.value}
            onChange={fader.onChange}
            channel="q"
            height={92}
            label=""
            archetype=""
          />
        )}
      </div>

      {/* Wire chip + jacks */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4, alignItems: 'center', width: '100%', paddingTop: 2, borderTop: '1px solid rgba(120,180,255,0.08)' }}>
        <span className="wire-pip" style={{ '--wire': wireColor }}>{wire}</span>
        {jacks && (
          <div style={{ display: 'flex', gap: 4 }}>
            {jacks.map((j, i) => (
              <Jack key={i} id={j.id} kind={j.kind} channel="q"/>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ── STANDARD COMPONENTS 4x4 GRID ─────────────────────────────────────
// Renders the 16 Standard Components of a module as a clickable 4x4 grid
// with wire-type color pips per cell.
function StdComponentGrid({ components = [], activeIdx, onSelect }) {
  return (
    <div className="std-cmp-grid" style={{ width: '100%' }}>
      {components.map((c, i) => (
        <div
          key={i}
          className={`std-cmp-cell ${activeIdx === i ? 'active' : ''}`}
          style={{ '--wire': `var(--wire-${c.wire})` }}
          onClick={() => onSelect?.(i)}
          title={`${c.name} · ${c.symbol} · ${c.wire}`}
        >
          {c.symbol}
        </div>
      ))}
    </div>
  );
}

// ── GENESIS MAP — 16 modules grouped by 4 departments ────────────────
const GENESIS_REGISTRY = [
  // Dept I — World Construction (cyan)
  { dept: 'I',  deptName: 'WORLD CONSTRUCTION', deptColor: '#1e8cff', mods: [
    { n: '01', name: 'Terrain',     crest: 'Atlas',     color: '#1e8cff', wire: 'SPA', built: true },
    { n: '02', name: 'Environment', crest: 'Mythos',    color: '#8050e0', wire: 'SPA', built: true },
    { n: '03', name: 'Architect',   crest: 'Architect', color: '#64b4ff', wire: 'SPA', built: true },
    { n: '04', name: 'Lighting',    crest: 'Prism',     color: '#e0d8ff', wire: 'VIS', built: true },
  ]},
  // Dept II — Entity Systems (gold)
  { dept: 'II', deptName: 'ENTITY SYSTEMS', deptColor: '#f4c025', mods: [
    { n: '05', name: 'Modeling',     crest: 'Animus',   color: '#f4c025', wire: 'VIS', built: true },
    { n: '06', name: 'Choreography', crest: 'Loom',     color: '#dc3c78', wire: 'BHV', built: true },
    { n: '07', name: 'Behavior',     crest: 'Instinct', color: '#9030d0', wire: 'BHV', built: false },
    { n: '08', name: 'Society',      crest: 'Order',    color: '#c8a860', wire: 'SOC', built: true },
  ]},
  // Dept III — Narrative Systems (violet)
  { dept: 'III', deptName: 'NARRATIVE SYSTEMS', deptColor: '#8c50ff', mods: [
    { n: '09', name: 'Sequencer', crest: 'Chronicle', color: '#b08030', wire: 'TMP', built: true },
    { n: '10', name: 'Story',     crest: 'Quill',     color: '#8c50ff', wire: 'NAR', built: true },
    { n: '11', name: 'Memory',    crest: 'Codex',     color: '#00c060', wire: 'NAR', built: true },
    { n: '12', name: 'Sound',     crest: 'Composer',  color: '#dc8c1e', wire: 'AUD', built: true },
  ]},
  // Dept IV — Pipeline Systems (bio)
  { dept: 'IV', deptName: 'PIPELINE SYSTEMS', deptColor: '#30e060', mods: [
    { n: '13', name: 'Logic',      crest: 'Axiom',     color: '#20c8d0', wire: 'LGC', built: false },
    { n: '14', name: 'Simulation', crest: 'Continuum', color: '#30e060', wire: 'DAT', built: false },
    { n: '15', name: 'Forge',      crest: 'Forge',     color: '#ff6400', wire: 'AST', built: true },
    { n: '16', name: 'Network',    crest: 'Nexus',     color: '#ffffff', wire: 'EVT', built: true },
  ]},
];

function GenesisMap({ activeModule, onSelect }) {
  return (
    <div className="genesis-map">
      {GENESIS_REGISTRY.map((d, di) => (
        <div key={d.dept} className="gm-dept" style={{ '--dept-color': d.deptColor, '--dept-glow': `${d.deptColor}66` }}>
          <div className="gm-dept-head">
            <div className="gm-dept-name">DEPT {d.dept} · {d.deptName}</div>
            <div className="gm-dept-tag">4MOD</div>
          </div>
          <div className="gm-modules">
            {d.mods.map(m => (
              <div
                key={m.n}
                className={`gm-mod ${m.built ? 'built' : 'in-progress'} ${activeModule === m.n ? 'active' : ''}`}
                style={{ '--mod-color': m.color }}
                onClick={() => onSelect?.(m.n)}
              >
                <span className="gm-num">{m.n}</span>
                <span className="gm-crest"/>
                <span style={{ fontFamily: 'var(--font-header)', letterSpacing: '0.14em', fontWeight: 500 }}>{m.name.toUpperCase()}</span>
                <span style={{ fontFamily: 'var(--font-code)', fontSize: 7, color: 'var(--fg-muted)', letterSpacing: '0.1em' }}>· {m.crest}</span>
                <span className="gm-status">{m.built ? '✓' : '◐'}</span>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

// ── HIERARCHY BREADCRUMB ─────────────────────────────────────────────
function HierCrumb({ active = 'mythos', stats }) {
  const tiers = [
    { id: 'genesis', label: 'GENESIS', meta: '1 sealed' },
    { id: 'mythos',  label: 'MYTHOS',  meta: '16 modules' },
    { id: 'standard', label: 'STANDARD', meta: '256 components' },
    { id: 'capsules', label: 'CAPSULES', meta: '4096 sealed' },
  ];
  return (
    <div className="hier-crumb">
      {tiers.map((t, i) => (
        <React.Fragment key={t.id}>
          <span className={`seg ${active === t.id ? 'active' : ''}`}>
            {t.label}
            <span style={{ color: 'var(--fg-muted)', marginLeft: 6, fontSize: 7 }}>· {t.meta}</span>
          </span>
          {i < tiers.length - 1 && <span className="arrow">▸</span>}
        </React.Fragment>
      ))}
      {stats && <span className="stat">{stats}</span>}
    </div>
  );
}

// ── EXPORT to window ─────────────────────────────────────────────────
Object.assign(window, {
  SignalContext, useSignal,
  Knob, Fader, Jack, SeqStep,
  Lissajous, Aurora, JogWheel, ResonanceMeter,
  Constellation, Lattice, MythosMap, HarmonicMeters,
  useLore, CablesLayer,
  GlyphSigil, GlyphHerald,
  ChannelStrip, StdComponentGrid, GenesisMap, HierCrumb,
  GENESIS_REGISTRY,
});
