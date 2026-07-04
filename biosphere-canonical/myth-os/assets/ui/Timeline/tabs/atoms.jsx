/* global React */
const { useState, useRef, useEffect, useMemo, useCallback } = React;

/* ═══════════════════════════════════════════════════════════════════
   AETHYR CONTROL ATOMS
   - Knob (5 silhouettes: classic, ringed, indexed, hatched, gilded)
   - Fader (vertical / horizontal)
   - Jack (input / output, with cable indicator)
   - Switch (toggle, 3-way, momentary)
   - Button (round, square, illuminated)
   - LED (single, ladder, ring)
   - SegmentDisplay (numeric / alpha)
   - Screw (M2 hex)
═══════════════════════════════════════════════════════════════════ */

// channel utility -> CSS custom props for accent + glow
function channelVars(channel = 'audio') {
  const map = {
    audio:  { c: 'var(--sig-audio)',  g: 'var(--quantum-glow)' },
    cv:     { c: 'var(--sig-cv)',     g: 'var(--mythos-glow)' },
    gate:   { c: 'var(--sig-gate)',   g: 'var(--gold-glow)' },
    trig:   { c: 'var(--sig-trig)',   g: 'var(--ember-glow)' },
    clock:  { c: 'var(--sig-clock)',  g: 'var(--rose-glow)' },
    poly:   { c: 'var(--sig-poly)',   g: 'var(--bio-glow)' },
    midi:   { c: 'var(--sig-midi)',   g: 'rgba(110,140,255,0.4)' },
    stream: { c: 'var(--sig-stream)', g: 'rgba(42,212,200,0.4)' },
    bio:    { c: 'var(--bio)',        g: 'var(--bio-glow)' },
    quantum:{ c: 'var(--quantum)',    g: 'var(--quantum-glow)' },
    mythos: { c: 'var(--mythos)',     g: 'var(--mythos-glow)' },
    gold:   { c: 'var(--gold)',       g: 'var(--gold-glow)' },
    ember:  { c: 'var(--ember)',      g: 'var(--ember-glow)' },
    rose:   { c: 'var(--rose)',       g: 'var(--rose-glow)' },
  };
  return map[channel] || map.audio;
}

/* ─────────────────────────────────────────────────────────────────
   Knob — drag to rotate. value 0-1.
   variants: classic | indexed | hatched | gilded | ringed
   sizes: 28 | 36 | 48 | 64 | 80
───────────────────────────────────────────────────────────────── */
function Knob({
  value = 0.62, onChange, size = 48, variant = 'classic',
  channel = 'audio', label, readout, min = 0, max = 1, unit = '',
  bipolar = false, locked = false,
}) {
  const [v, setV] = useState(value);
  const ref = useRef(null);
  const dragRef = useRef(null);
  const cur = onChange ? value : v;
  const setVal = (next) => {
    next = Math.max(0, Math.min(1, next));
    if (onChange) onChange(next);
    else setV(next);
  };
  const startDrag = (e) => {
    e.preventDefault();
    if (locked) return;
    dragRef.current = { y: e.clientY, v: cur };
    const move = (ev) => {
      const dy = dragRef.current.y - ev.clientY;
      setVal(dragRef.current.v + dy / 180);
    };
    const up = () => {
      window.removeEventListener('mousemove', move);
      window.removeEventListener('mouseup', up);
    };
    window.addEventListener('mousemove', move);
    window.addEventListener('mouseup', up);
  };

  const ch = channelVars(channel);
  // sweep angle: -135 to +135 (270° total)
  const sweep = bipolar ? (cur - 0.5) * 270 : -135 + cur * 270;
  const arcStart = bipolar ? 0 : -135;
  const arcEnd = bipolar ? sweep : sweep;
  const displayValue = readout != null ? readout : (
    bipolar
      ? ((cur - 0.5) * 2 * (max - min) / 2).toFixed(2)
      : (min + cur * (max - min)).toFixed(2)
  );

  // Build SVG arc indicator (the lit ring)
  const cx = size/2, cy = size/2, r = size/2 - 3;
  const polar = (deg) => {
    const rad = (deg - 90) * Math.PI / 180;
    return [cx + r * Math.cos(rad), cy + r * Math.sin(rad)];
  };
  const arcPath = (a0, a1) => {
    const [x0, y0] = polar(a0);
    const [x1, y1] = polar(a1);
    const large = Math.abs(a1 - a0) > 180 ? 1 : 0;
    const sweepFlag = a1 > a0 ? 1 : 0;
    return `M ${x0} ${y0} A ${r} ${r} 0 ${large} ${sweepFlag} ${x1} ${y1}`;
  };

  const knobBody = (
    <div
      ref={ref}
      onMouseDown={startDrag}
      style={{
        width: size, height: size,
        position: 'relative',
        borderRadius: '50%',
        cursor: locked ? 'not-allowed' : 'grab',
        userSelect: 'none',
        '--d-color': ch.c, '--d-glow': ch.g,
      }}
    >
      {/* track ring */}
      <svg
        viewBox={`0 0 ${size} ${size}`}
        style={{ position: 'absolute', inset: 0, overflow: 'visible' }}
      >
        <path d={arcPath(-135, 135)}
          fill="none" stroke="rgba(120,180,255,0.08)" strokeWidth="2" strokeLinecap="round" />
        <path d={arcPath(arcStart, arcEnd)}
          fill="none" stroke={ch.c} strokeWidth="2" strokeLinecap="round"
          style={{ filter: `drop-shadow(0 0 4px ${ch.g})` }} />

        {variant === 'indexed' && Array.from({length: 11}).map((_, i) => {
          const a = -135 + i * 27;
          const [x0,y0] = polar(a);
          const innerR = r - 4;
          const x1 = cx + innerR * Math.cos((a-90)*Math.PI/180);
          const y1 = cy + innerR * Math.sin((a-90)*Math.PI/180);
          const lit = bipolar ? (a >= 0 ? a <= sweep : a >= sweep) : (a <= sweep);
          return <line key={i} x1={x1} y1={y1} x2={x0} y2={y0}
            stroke={lit ? ch.c : 'rgba(120,180,255,0.18)'} strokeWidth="1.2"
            style={lit ? { filter: `drop-shadow(0 0 3px ${ch.g})` } : {}} />;
        })}

        {variant === 'hatched' && Array.from({length: 32}).map((_, i) => {
          const a = i * (360/32);
          const innerR = r - 2;
          const [x0,y0] = polar(a);
          const x1 = cx + innerR * Math.cos((a-90)*Math.PI/180);
          const y1 = cy + innerR * Math.sin((a-90)*Math.PI/180);
          return <line key={i} x1={x1} y1={y1} x2={x0} y2={y0}
            stroke="rgba(255,255,255,0.12)" strokeWidth="0.6" />;
        })}
      </svg>

      {/* cap */}
      <div style={{
        position: 'absolute',
        inset: variant === 'gilded' ? 6 : 5,
        borderRadius: '50%',
        background: variant === 'gilded'
          ? `radial-gradient(circle at 35% 25%, rgba(255,235,180,0.4) 0%, rgba(180,130,30,0.15) 30%, #0a0c12 75%), conic-gradient(from 0deg, #c8920a, #fbbf24, #c8920a, #8b5a0a, #c8920a)`
          : 'var(--knob-cap)',
        boxShadow: `
          inset 0 1px 1px rgba(255,255,255,0.15),
          inset 0 -2px 4px rgba(0,0,0,0.5),
          0 2px 6px rgba(0,0,0,0.5)
        `,
        transform: `rotate(${sweep}deg)`,
        transition: dragRef.current ? 'none' : 'transform 80ms var(--ease-out)',
      }}>
        {/* indicator notch */}
        <div style={{
          position: 'absolute',
          left: '50%', top: '8%',
          width: 2, height: '28%',
          background: variant === 'gilded' ? '#fff4c0' : ch.c,
          borderRadius: 1,
          transform: 'translateX(-50%)',
          boxShadow: `0 0 4px ${ch.g}`,
        }} />
      </div>

      {/* center dot for ringed variant */}
      {variant === 'ringed' && (
        <div style={{
          position: 'absolute',
          inset: '38%',
          borderRadius: '50%',
          background: ch.c,
          boxShadow: `0 0 12px ${ch.g}, inset 0 0 4px rgba(0,0,0,0.4)`,
          opacity: 0.85,
        }} />
      )}
    </div>
  );

  return (
    <div style={{ display: 'inline-flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}>
      {knobBody}
      {label && <div className="cap-label" style={{marginTop: 2}}>{label}</div>}
      {readout !== false && (
        <div style={{
          fontFamily: 'var(--font-code)', fontSize: 9,
          color: ch.c, letterSpacing: '0.05em',
          textShadow: `0 0 4px ${ch.g}`,
        }}>{displayValue}{unit}</div>
      )}
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────
   Fader — vertical or horizontal slider, with track LEDs
───────────────────────────────────────────────────────────────── */
function Fader({
  value = 0.55, onChange, length = 120, orient = 'v',
  channel = 'audio', label, readout, ledTrack = true,
}) {
  const [v, setV] = useState(value);
  const trackRef = useRef(null);
  const cur = onChange ? value : v;
  const ch = channelVars(channel);

  const startDrag = (e) => {
    e.preventDefault();
    const rect = trackRef.current.getBoundingClientRect();
    const update = (ev) => {
      const pct = orient === 'v'
        ? 1 - (ev.clientY - rect.top) / rect.height
        : (ev.clientX - rect.left) / rect.width;
      const next = Math.max(0, Math.min(1, pct));
      if (onChange) onChange(next); else setV(next);
    };
    update(e);
    const move = (ev) => update(ev);
    const up = () => {
      window.removeEventListener('mousemove', move);
      window.removeEventListener('mouseup', up);
    };
    window.addEventListener('mousemove', move);
    window.addEventListener('mouseup', up);
  };

  const trackW = orient === 'v' ? 16 : length;
  const trackH = orient === 'v' ? length : 16;

  return (
    <div style={{ display: 'inline-flex', flexDirection: 'column', alignItems: 'center', gap: 6 }}>
      <div
        ref={trackRef}
        onMouseDown={startDrag}
        style={{
          width: trackW, height: trackH,
          background: 'var(--panel-inset)',
          borderRadius: 3,
          border: '1px solid rgba(0,0,0,0.6)',
          boxShadow: 'inset 0 2px 4px rgba(0,0,0,0.6), 0 1px 0 rgba(255,255,255,0.04)',
          position: 'relative',
          cursor: 'ns-resize',
          userSelect: 'none',
        }}
      >
        {/* lit track */}
        {ledTrack && (
          <div style={{
            position: 'absolute',
            ...(orient === 'v'
              ? { left: 2, right: 2, bottom: 2, height: `calc(${cur*100}% - 4px)` }
              : { top: 2, bottom: 2, left: 2, width: `calc(${cur*100}% - 4px)` }),
            background: `linear-gradient(${orient === 'v' ? '0deg' : '90deg'},
              ${ch.c} 0%, ${ch.c} 60%, transparent 100%)`,
            opacity: 0.7,
            borderRadius: 2,
            boxShadow: `0 0 8px ${ch.g}`,
          }} />
        )}
        {/* notch ticks */}
        {Array.from({length: 9}).map((_,i) => (
          <div key={i} style={{
            position: 'absolute',
            ...(orient === 'v'
              ? { left: -3, right: -3, top: `${i*12.5}%`, height: 1 }
              : { top: -3, bottom: -3, left: `${i*12.5}%`, width: 1 }),
            background: i % 4 === 0 ? 'rgba(180,200,220,0.25)' : 'rgba(120,150,180,0.12)',
          }} />
        ))}
        {/* cap */}
        <div style={{
          position: 'absolute',
          ...(orient === 'v'
            ? {
                left: -4, right: -4,
                top: `calc(${(1-cur)*100}% - 8px)`,
                height: 16,
              }
            : {
                top: -4, bottom: -4,
                left: `calc(${cur*100}% - 8px)`,
                width: 16,
              }),
          background: 'linear-gradient(180deg, #2a313f 0%, #0e1118 100%)',
          borderRadius: 2,
          boxShadow: `
            inset 0 1px 0 rgba(255,255,255,0.2),
            inset 0 -1px 0 rgba(0,0,0,0.4),
            0 2px 4px rgba(0,0,0,0.6),
            0 0 6px ${ch.g}
          `,
          border: `1px solid ${ch.c}`,
          borderColor: 'rgba(0,229,255,0.3)',
        }}>
          <div style={{
            position: 'absolute',
            ...(orient === 'v'
              ? { top: '50%', left: 2, right: 2, height: 1 }
              : { left: '50%', top: 2, bottom: 2, width: 1 }),
            background: ch.c,
            boxShadow: `0 0 4px ${ch.g}`,
          }} />
        </div>
      </div>
      {label && <div className="cap-label">{label}</div>}
      {readout && <div style={{
        fontFamily: 'var(--font-code)', fontSize: 9, color: ch.c,
        textShadow: `0 0 4px ${ch.g}`, letterSpacing: '0.05em',
      }}>{readout}</div>}
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────
   Jack — input or output port. Shows cable when patched.
───────────────────────────────────────────────────────────────── */
function Jack({ channel = 'audio', label, dir = 'in', patched = false, size = 22 }) {
  const ch = channelVars(channel);
  return (
    <div style={{ display: 'inline-flex', flexDirection: 'column', alignItems: 'center', gap: 3 }}>
      <div style={{
        width: size, height: size,
        borderRadius: '50%',
        background: 'var(--jack-ring)',
        boxShadow: `
          inset 0 1px 1px rgba(255,255,255,0.08),
          inset 0 -1px 1px rgba(0,0,0,0.4),
          0 1px 2px rgba(0,0,0,0.6),
          0 0 0 1px ${ch.c}33,
          ${patched ? `0 0 8px ${ch.g}` : 'none'}
        `,
        position: 'relative',
        border: `1px solid ${dir === 'out' ? ch.c : 'rgba(120,180,255,0.25)'}`,
      }}>
        <div style={{
          position: 'absolute', inset: 4,
          borderRadius: '50%',
          background: 'var(--jack-hole)',
          boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.9)',
        }}>
          {patched && (
            <div style={{
              position: 'absolute', inset: 3,
              borderRadius: '50%',
              background: ch.c,
              opacity: 0.9,
              boxShadow: `0 0 6px ${ch.g}`,
            }} />
          )}
        </div>
      </div>
      {label && <div style={{
        fontFamily: 'var(--font-code)', fontSize: 8,
        letterSpacing: '0.18em', textTransform: 'uppercase',
        color: dir === 'out' ? ch.c : 'var(--fg-3)',
      }}>{label}</div>}
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────
   Switch — 2-way or 3-way
───────────────────────────────────────────────────────────────── */
function Switch({ value = 0, onChange, options = ['OFF', 'ON'], orient = 'v', channel = 'gold', label }) {
  const [v, setV] = useState(value);
  const cur = onChange ? value : v;
  const ch = channelVars(channel);
  const set = (i) => onChange ? onChange(i) : setV(i);

  const len = options.length;
  return (
    <div style={{ display: 'inline-flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}>
      <div style={{
        background: 'var(--panel-inset)',
        borderRadius: 3,
        border: '1px solid rgba(0,0,0,0.5)',
        padding: 2,
        display: 'flex',
        flexDirection: orient === 'v' ? 'column' : 'row',
        boxShadow: 'inset 0 1px 3px rgba(0,0,0,0.6)',
      }}>
        {options.map((o, i) => (
          <button key={i} onClick={() => set(i)} style={{
            background: cur === i
              ? `linear-gradient(180deg, ${ch.c}40 0%, ${ch.c}10 100%)`
              : 'transparent',
            border: 'none',
            borderRadius: 2,
            padding: orient === 'v' ? '4px 10px' : '4px 8px',
            color: cur === i ? ch.c : 'var(--fg-3)',
            fontFamily: 'var(--font-code)',
            fontSize: 9, letterSpacing: '0.15em',
            cursor: 'pointer',
            textShadow: cur === i ? `0 0 4px ${ch.g}` : 'none',
            minWidth: 28,
          }}>{o}</button>
        ))}
      </div>
      {label && <div className="cap-label">{label}</div>}
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────
   IButton — illuminated round button
───────────────────────────────────────────────────────────────── */
function IButton({ on = false, onClick, channel = 'audio', size = 32, icon, label, momentary = false }) {
  const [pressed, setPressed] = useState(false);
  const lit = momentary ? pressed : on;
  const ch = channelVars(channel);
  return (
    <div style={{ display: 'inline-flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}>
      <button
        onMouseDown={() => { setPressed(true); if (onClick) onClick(); }}
        onMouseUp={() => setPressed(false)}
        onMouseLeave={() => setPressed(false)}
        style={{
          width: size, height: size,
          borderRadius: '50%',
          background: lit
            ? `radial-gradient(circle at 35% 30%, ${ch.c} 0%, ${ch.c}aa 30%, #0a0c12 80%)`
            : `radial-gradient(circle at 35% 30%, #2a313f 0%, #0e1118 60%, #0a0c12 100%)`,
          border: `1px solid ${lit ? ch.c : 'rgba(120,180,255,0.15)'}`,
          boxShadow: lit
            ? `inset 0 1px 2px rgba(255,255,255,0.3),
               inset 0 -2px 4px rgba(0,0,0,0.4),
               0 0 12px ${ch.g}, 0 0 24px ${ch.g}`
            : `inset 0 1px 1px rgba(255,255,255,0.08),
               inset 0 -2px 3px rgba(0,0,0,0.4),
               0 2px 4px rgba(0,0,0,0.6)`,
          color: lit ? '#000' : 'var(--fg-3)',
          fontSize: 12,
          cursor: 'pointer',
          padding: 0,
          display: 'grid', placeItems: 'center',
          transition: 'all 120ms var(--ease-out)',
          transform: pressed ? 'scale(0.96)' : 'scale(1)',
        }}>
        {icon}
      </button>
      {label && <div className="cap-label">{label}</div>}
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────
   LED — single tiny indicator
───────────────────────────────────────────────────────────────── */
function LED({ on = true, channel = 'audio', size = 6, blink = false }) {
  const ch = channelVars(channel);
  return (
    <div style={{
      width: size, height: size, borderRadius: '50%',
      background: on ? ch.c : 'var(--led-off)',
      boxShadow: on ? `0 0 ${size}px ${ch.g}, inset 0 0 ${size/3}px rgba(255,255,255,0.4)` : 'inset 0 1px 2px rgba(0,0,0,0.6)',
      animation: blink && on ? 'pulse-led 1s ease-in-out infinite' : 'none',
      flexShrink: 0,
    }} />
  );
}

/* ─────────────────────────────────────────────────────────────────
   LEDLadder — vertical bar of LEDs (VU-style)
───────────────────────────────────────────────────────────────── */
function LEDLadder({ count = 12, level = 0.6, channel = 'bio', warn = 0.75, danger = 0.9, orient = 'v' }) {
  return (
    <div style={{
      display: 'flex',
      flexDirection: orient === 'v' ? 'column-reverse' : 'row',
      gap: 2,
      padding: 3,
      background: 'var(--panel-inset)',
      borderRadius: 3,
      border: '1px solid rgba(0,0,0,0.5)',
      boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.6)',
    }}>
      {Array.from({length: count}).map((_, i) => {
        const t = (i + 1) / count;
        const lit = t <= level;
        const seg = t > danger ? 'ember' : t > warn ? 'gold' : channel;
        return <div key={i} style={{
          ...(orient === 'v' ? { width: 14, height: 4 } : { width: 4, height: 14 }),
          background: lit ? `var(--sig-${seg === 'bio' ? 'poly' : seg === 'ember' ? 'trig' : 'gate'})` : '#0a0c12',
          borderRadius: 1,
          boxShadow: lit ? `0 0 4px ${seg === 'ember' ? 'var(--ember-glow)' : seg === 'gold' ? 'var(--gold-glow)' : 'var(--bio-glow)'}` : 'inset 0 1px 1px rgba(0,0,0,0.6)',
          opacity: lit ? 1 : 0.4,
        }} />;
      })}
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────
   SegmentDisplay — 7-seg numeric / alpha
───────────────────────────────────────────────────────────────── */
function SegmentDisplay({ text = '888', channel = 'ember', width, height = 28 }) {
  const ch = channelVars(channel);
  return (
    <div style={{
      width: width || 'auto',
      height,
      padding: '4px 10px',
      background: 'linear-gradient(180deg, #050709 0%, #0a0c12 100%)',
      border: '1px solid rgba(0,0,0,0.7)',
      borderRadius: 3,
      boxShadow: 'inset 0 2px 4px rgba(0,0,0,0.7), 0 1px 0 rgba(255,255,255,0.03)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      fontFamily: 'var(--font-code)',
      fontWeight: 700,
      fontSize: height * 0.6,
      color: ch.c,
      letterSpacing: '0.15em',
      textShadow: `0 0 6px ${ch.g}, 0 0 2px ${ch.g}`,
      position: 'relative',
      overflow: 'hidden',
    }}>
      <span style={{ position: 'absolute', opacity: 0.08 }}>{'8'.repeat(String(text).length)}</span>
      <span style={{ position: 'relative' }}>{text}</span>
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────
   Screw — M2 hex, decorative
───────────────────────────────────────────────────────────────── */
function Screw({ size = 8, angle = 30 }) {
  return (
    <div style={{
      width: size, height: size,
      borderRadius: '50%',
      background: 'radial-gradient(circle at 30% 25%, #4a5060 0%, #1a1f2a 60%, #0a0c12 100%)',
      boxShadow: 'inset 0 0 1px rgba(255,255,255,0.15), 0 1px 1px rgba(0,0,0,0.6)',
      position: 'relative',
    }}>
      <div style={{
        position: 'absolute', inset: '20%',
        background: 'rgba(0,0,0,0.55)',
        transform: `rotate(${angle}deg)`,
        clipPath: 'polygon(45% 0, 55% 0, 55% 100%, 45% 100%)',
      }} />
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────
   Patch wire — bezier SVG between two anchors
───────────────────────────────────────────────────────────────── */
function PatchWire({ from, to, channel = 'audio', flow = true }) {
  const ch = channelVars(channel);
  const dx = (to.x - from.x);
  const dy = Math.abs(to.y - from.y);
  const sag = Math.max(40, dy * 0.5 + Math.abs(dx) * 0.1);
  const c1x = from.x; const c1y = from.y + sag;
  const c2x = to.x;   const c2y = to.y + sag;
  const path = `M ${from.x} ${from.y} C ${c1x} ${c1y}, ${c2x} ${c2y}, ${to.x} ${to.y}`;
  return (
    <g>
      <path d={path} fill="none" stroke="#000" strokeWidth="6" strokeLinecap="round" opacity="0.5" />
      <path d={path} fill="none" stroke={ch.c} strokeWidth="3" strokeLinecap="round"
        style={{ filter: `drop-shadow(0 0 4px ${ch.g})`, opacity: 0.85 }} />
      {flow && (
        <path d={path} fill="none" stroke="rgba(255,255,255,0.6)" strokeWidth="1.5"
          strokeDasharray="2 10" strokeLinecap="round"
          style={{ animation: 'wire-pulse 0.8s linear infinite' }} />
      )}
    </g>
  );
}

Object.assign(window, {
  channelVars,
  Knob, Fader, Jack, Switch, IButton, LED, LEDLadder, SegmentDisplay, Screw, PatchWire,
});
