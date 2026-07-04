/* Quill OS — shared components (React, via Babel).
   IMPORTANT: Uses uniquely-named style objects to avoid global collisions. */

const { useState, useEffect, useRef, useMemo } = React;

// ─── Corner circuit-marks wrapper ──────────────────────────────────
function Corners({ children, tone = 'gold', className = '', style = {} }) {
  const color = tone === 'quantum' ? 'rgba(0,229,255,0.55)'
               : tone === 'bio'     ? 'rgba(57,255,20,0.55)'
               : tone === 'mythos'  ? 'rgba(192,132,252,0.55)'
               : 'rgba(251,191,36,0.55)';
  const markStyle = (pos) => ({
    position: 'absolute', width: 12, height: 12, pointerEvents: 'none',
    ...(pos === 'tl' ? { top: 4, left: 4, borderTop: `1.5px solid ${color}`, borderLeft: `1.5px solid ${color}` } : {}),
    ...(pos === 'br' ? { bottom: 4, right: 4, borderBottom: `1.5px solid ${color}`, borderRight: `1.5px solid ${color}` } : {}),
  });
  return (
    <div className={className} style={{ position: 'relative', ...style }}>
      <span style={markStyle('tl')} /><span style={markStyle('br')} />
      {children}
    </div>
  );
}

// ─── Crest — entity heraldic emblem ───────────────────────────────
function Crest({ entity, size = 120 }) {
  const { color, glow, shape, glyph, motto, code } = entity;
  return (
    <div style={{ textAlign: 'center' }}>
      <div className="checker" style={{
        width: size + 20, height: size + 20, borderRadius: 6, padding: 10,
        margin: '0 auto', position: 'relative',
      }}>
        <svg width={size} height={size} viewBox="0 0 120 120" style={{ display: 'block' }}>
          <defs>
            <radialGradient id={`g-${code}`} cx="50%" cy="45%" r="55%">
              <stop offset="0%" stopColor={color} stopOpacity="0.75"/>
              <stop offset="55%" stopColor={color} stopOpacity="0.2"/>
              <stop offset="100%" stopColor="#03050a" stopOpacity="0"/>
            </radialGradient>
          </defs>
          <g style={{ transformOrigin: '60px 60px', animation: 'spin-slow 18s linear infinite' }}>
            <circle cx="60" cy="60" r="54" fill="none" stroke={color} strokeWidth="0.7" strokeDasharray="2 3" opacity="0.55"/>
          </g>
          <circle cx="60" cy="60" r="48" fill="none" stroke={color} strokeWidth="1.1"/>
          {shape === 'shield' && (
            <path d="M60 18 L96 28 L96 62 Q96 92 60 104 Q24 92 24 62 L24 28 Z"
                  fill={`url(#g-${code})`} stroke={color} strokeWidth="1.2"/>
          )}
          {shape === 'hex' && (
            <polygon points="60,16 96,36 96,80 60,100 24,80 24,36"
                  fill={`url(#g-${code})`} stroke={color} strokeWidth="1.2"/>
          )}
          {shape === 'rose' && (
            <g>
              <path d="M60 22 C30 38 30 82 60 98 C90 82 90 38 60 22 Z" fill={`url(#g-${code})`} stroke={color} strokeWidth="1.2"/>
              <path d="M20 60 C36 30 84 30 100 60 C84 90 36 90 20 60 Z" fill="none" stroke={color} strokeWidth="0.9" opacity="0.6"/>
            </g>
          )}
          <text x="60" y="66" textAnchor="middle" fontFamily="Cinzel, serif"
                fontWeight="700" fontSize="18" fill={color} letterSpacing="2">{glyph}</text>
          <circle cx="60" cy="82" r="4" fill={color}
                  style={{ transformOrigin: '60px 82px', animation: 'pulse-ring 3s ease-in-out infinite' }}/>
        </svg>
      </div>
      <div style={{
        fontFamily: 'Cinzel, serif', fontSize: 12, letterSpacing: '0.2em',
        color: color, marginTop: 10, textTransform: 'uppercase',
      }}>{entity.name}</div>
      <div style={{
        fontFamily: 'JetBrains Mono, monospace', fontSize: 9,
        color: 'rgba(100,116,139,0.8)', letterSpacing: '0.18em', marginTop: 3,
      }}>{code} · {entity.hz}HZ</div>
      <div style={{
        fontFamily: 'Cormorant Garamond, serif', fontStyle: 'italic',
        fontSize: 11, color: 'rgba(148,163,184,0.7)', marginTop: 6, maxWidth: size + 40,
        marginLeft: 'auto', marginRight: 'auto', lineHeight: 1.4,
      }}>"{motto}"</div>
    </div>
  );
}

// ─── Node — small instrument glyph ────────────────────────────────
function Node({ entity, size = 72, live = false }) {
  const { color, shape, code } = entity;
  return (
    <div className="checker" style={{
      width: size + 16, height: size + 16, padding: 8, borderRadius: 4,
      display: 'inline-block',
    }}>
      <svg width={size} height={size} viewBox="0 0 64 64">
        <g style={{ transformOrigin: '32px 32px', animation: 'spin-slow 16s linear infinite' }}>
          <circle cx="32" cy="32" r="28" fill="none" stroke={color} strokeWidth="0.6" strokeDasharray="2 3" opacity="0.5"/>
        </g>
        {shape === 'shield' && <polygon points="32,10 52,22 52,46 32,56 12,46 12,22" fill="none" stroke={color} strokeWidth="1.3"/>}
        {shape === 'hex' && <polygon points="32,10 54,22 54,42 32,54 10,42 10,22" fill="none" stroke={color} strokeWidth="1.3"/>}
        {shape === 'rose' && <path d="M32 10 C14 22 14 42 32 54 C50 42 50 22 32 10 Z" fill="none" stroke={color} strokeWidth="1.3"/>}
        <circle cx="32" cy="32" r="3" fill={color}
                style={{ transformOrigin: '32px 32px', animation: live ? 'pulse-ring 1.6s ease-in-out infinite' : 'pulse-ring 4s ease-in-out infinite' }}/>
      </svg>
    </div>
  );
}

// ─── Knob ─────────────────────────────────────────────────────────
function Knob({ label, value, min = 0, max = 1, tone = 'quantum', size = 56, onChange }) {
  const color = tone === 'quantum' ? '#00e5ff' : tone === 'bio' ? '#39ff14' : tone === 'mythos' ? '#c084fc' : '#fbbf24';
  const glow  = tone === 'quantum' ? 'rgba(0,229,255,0.35)' : tone === 'bio' ? 'rgba(57,255,20,0.35)' : tone === 'mythos' ? 'rgba(192,132,252,0.35)' : 'rgba(251,191,36,0.35)';
  const pct = (value - min) / (max - min);
  const rot = -135 + pct * 270;
  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6 }}>
      <div
        onWheel={(e) => {
          if (!onChange) return;
          e.preventDefault();
          const step = (max - min) / 40;
          const next = Math.max(min, Math.min(max, value - Math.sign(e.deltaY) * step));
          onChange(next);
        }}
        style={{
          width: size, height: size, borderRadius: '50%',
          background: 'radial-gradient(circle at 30% 30%, #243044, #07090f)',
          border: `1px solid ${color}`,
          boxShadow: `0 0 10px ${glow}, inset 0 0 8px rgba(0,0,0,0.6)`,
          position: 'relative', cursor: 'ns-resize',
        }}
      >
        <div style={{
          position: 'absolute', top: 4, left: '50%', width: 2, height: size * 0.3,
          background: color, boxShadow: `0 0 6px ${color}`,
          transform: `translateX(-50%) rotate(${rot}deg)`,
          transformOrigin: `center ${size * 0.46}px`,
        }}/>
      </div>
      <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 9, color, letterSpacing: '0.15em' }}>
        {label}
      </div>
      <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#64748b' }}>
        {value.toFixed(2)}
      </div>
    </div>
  );
}

// ─── Switch / slider ───────────────────────────────────────────────
function Toggle({ label, on, tone = 'gold', onChange }) {
  const color = tone === 'quantum' ? '#00e5ff' : tone === 'bio' ? '#39ff14' : tone === 'mythos' ? '#c084fc' : '#fbbf24';
  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6 }}>
      <div
        onClick={() => onChange && onChange(!on)}
        style={{
          width: 36, height: 56, borderRadius: 4, background: '#0d1117',
          border: `1px solid ${color}80`, position: 'relative',
          boxShadow: 'inset 0 0 8px rgba(0,0,0,0.6)', cursor: 'pointer',
          transition: 'all 180ms',
        }}
      >
        <div style={{
          position: 'absolute', left: 4, right: 4, height: 20,
          background: on ? `linear-gradient(${color}, ${color}aa)` : '#243044',
          borderRadius: 2,
          top: on ? 4 : 32,
          boxShadow: on ? `0 0 8px ${color}` : 'none',
          transition: 'all 220ms cubic-bezier(0.34,1.56,0.64,1)',
        }}/>
      </div>
      <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 9, color, letterSpacing: '0.15em' }}>{label}</div>
    </div>
  );
}

// ─── LED ──────────────────────────────────────────────────────────
function Led({ tone = 'bio', on = true, label }) {
  const color = tone === 'quantum' ? '#00e5ff' : tone === 'bio' ? '#39ff14' : tone === 'mythos' ? '#c084fc' : tone === 'ember' ? '#f97316' : '#fbbf24';
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
      <div style={{
        width: 8, height: 8, borderRadius: '50%',
        background: on ? color : '#243044',
        boxShadow: on ? `0 0 8px ${color}, 0 0 14px ${color}80` : 'none',
        animation: on ? 'led-blink 1.4s ease-in-out infinite' : 'none',
      }}/>
      {label && <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 9, color: '#94a3b8', letterSpacing: '0.15em' }}>{label}</span>}
    </div>
  );
}

// ─── Chip / tag ───────────────────────────────────────────────────
function Chip({ children, tone = 'quantum', solid = false }) {
  const color = tone === 'quantum' ? '#00e5ff' : tone === 'bio' ? '#39ff14' : tone === 'mythos' ? '#c084fc' : tone === 'ember' ? '#f97316' : '#fbbf24';
  return (
    <span style={{
      fontFamily: 'JetBrains Mono, monospace', fontSize: 9,
      letterSpacing: '0.2em', padding: '3px 8px', borderRadius: 2,
      color, border: `1px solid ${color}66`,
      background: solid ? `${color}14` : 'transparent',
      textTransform: 'uppercase', whiteSpace: 'nowrap',
    }}>{children}</span>
  );
}

// ─── Step sequencer ───────────────────────────────────────────────
function Sequencer({ steps, tone = 'quantum', active = -1 }) {
  const color = tone === 'quantum' ? '#00e5ff' : tone === 'bio' ? '#39ff14' : tone === 'mythos' ? '#c084fc' : '#fbbf24';
  return (
    <div style={{ display: 'grid', gridTemplateColumns: `repeat(${steps.length}, 1fr)`, gap: 3 }}>
      {steps.map((on, i) => (
        <div key={i} style={{
          height: 20, borderRadius: 1,
          background: on ? color : '#0d1117',
          border: `1px solid ${i === active ? color : (on ? color : 'rgba(0,229,255,0.12)')}`,
          boxShadow: on ? `0 0 6px ${color}80` : (i === active ? `0 0 8px ${color}` : 'none'),
          transition: 'all 120ms',
        }}/>
      ))}
    </div>
  );
}

// Expose globals so other babel scripts can see them
Object.assign(window, { Corners, Crest, Node, Knob, Toggle, Led, Chip, Sequencer });
