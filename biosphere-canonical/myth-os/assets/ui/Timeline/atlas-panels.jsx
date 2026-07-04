/* Quantum Atlas — Side Panels
   - ToolPalette (left strip)
   - LayerManager (right panel tab)
   - EntityInspector (right panel tab)
   - WorldCodex (right panel tab)
   - ScribeLog (right panel tab)
   Depends on: window.{Corners, Chip, Led, Knob, Toggle} from components.jsx
               window.REGIONS from atlas-map.jsx
*/

const { useState, useEffect, useRef } = React;

// ── Tool definitions ───────────────────────────────────────────────
const TOOLS = [
  { id: 'select',   glyph: '◈',  label: 'SELECT',   tone: 'quantum', key: 'V' },
  { id: 'region',   glyph: '⬡',  label: 'REGION',   tone: 'bio',     key: 'R' },
  { id: 'route',    glyph: '⟿',  label: 'ROUTE',    tone: 'quantum', key: 'L' },
  { id: 'annotate', glyph: '✦',  label: 'ANNOTATE', tone: 'mythos',  key: 'A' },
  { id: 'survey',   glyph: '⊕',  label: 'SURVEY',   tone: 'bio',     key: 'S' },
  { id: 'erase',    glyph: '◻',  label: 'ERASE',    tone: 'ember',   key: 'E' },
  { id: 'measure',  glyph: '⟺',  label: 'MEASURE',  tone: 'gold',    key: 'M' },
  { id: 'lore',     glyph: '𓂀',  label: 'LORE',     tone: 'mythos',  key: 'B' },
];

const TONE_COLOR = {
  quantum: '#00e5ff', bio: '#39ff14', mythos: '#c084fc',
  gold: '#fbbf24', ember: '#f97316',
};
const TONE_GLOW = {
  quantum: 'rgba(0,229,255,0.35)', bio: 'rgba(57,255,20,0.35)',
  mythos: 'rgba(192,132,252,0.35)', gold: 'rgba(251,191,36,0.35)',
  ember: 'rgba(249,115,22,0.35)',
};

// ── Tool Palette (left strip) ──────────────────────────────────────
function ToolPalette({ activeTool, onSelectTool }) {
  return (
    <div style={{
      width: 56,
      background: 'rgba(7,9,15,0.92)',
      backdropFilter: 'blur(12px)',
      borderRight: '1px solid rgba(120,180,255,0.08)',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      gap: 2,
      padding: '12px 0',
      flexShrink: 0,
    }}>
      {/* Logo mark */}
      <div style={{ marginBottom: 12, opacity: 0.7 }}>
        <div style={{
          width: 32, height: 32, borderRadius: 4,
          background: 'rgba(251,191,36,0.08)',
          border: '1px solid rgba(251,191,36,0.25)',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
        }}>
          <span style={{ fontFamily: 'Cinzel Decorative, serif', fontSize: 14, color: '#fbbf24' }}>Q</span>
        </div>
      </div>

      <div style={{ width: '80%', height: 1, background: 'rgba(251,191,36,0.15)', marginBottom: 8 }}/>

      {TOOLS.map(tool => {
        const active = activeTool === tool.id;
        const col = TONE_COLOR[tool.tone];
        return (
          <div
            key={tool.id}
            title={`${tool.label} [${tool.key}]`}
            onClick={() => onSelectTool(tool.id)}
            style={{
              width: 40, height: 40, borderRadius: 4,
              display: 'flex', flexDirection: 'column',
              alignItems: 'center', justifyContent: 'center',
              gap: 2, cursor: 'pointer',
              background: active ? `${col}18` : 'transparent',
              border: `1px solid ${active ? col : 'transparent'}`,
              boxShadow: active ? `0 0 10px ${TONE_GLOW[tool.tone]}` : 'none',
              transition: 'all 180ms cubic-bezier(0.22,1,0.36,1)',
            }}
            onMouseEnter={e => {
              if (!active) {
                e.currentTarget.style.background = `${col}0e`;
                e.currentTarget.style.borderColor = `${col}44`;
              }
            }}
            onMouseLeave={e => {
              if (!active) {
                e.currentTarget.style.background = 'transparent';
                e.currentTarget.style.borderColor = 'transparent';
              }
            }}
          >
            <span style={{ fontSize: 14, color: active ? col : '#475569', lineHeight: 1 }}>{tool.glyph}</span>
            <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 6, color: active ? col : '#334155', letterSpacing: '0.1em' }}>{tool.key}</span>
          </div>
        );
      })}

      {/* Bottom spacer tools */}
      <div style={{ flex: 1 }}/>
      <div style={{ width: '80%', height: 1, background: 'rgba(251,191,36,0.1)', marginBottom: 8 }}/>
      {[{ glyph: '⚙', label: 'CFG' }, { glyph: '◎', label: 'REF' }].map(t => (
        <div key={t.label} style={{
          width: 40, height: 40, borderRadius: 4,
          display: 'flex', flexDirection: 'column',
          alignItems: 'center', justifyContent: 'center',
          gap: 2, cursor: 'pointer', opacity: 0.45,
          transition: 'opacity 180ms',
        }}
          onMouseEnter={e => e.currentTarget.style.opacity = '0.8'}
          onMouseLeave={e => e.currentTarget.style.opacity = '0.45'}
        >
          <span style={{ fontSize: 13, color: '#64748b' }}>{t.glyph}</span>
          <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 6, color: '#334155' }}>{t.label}</span>
        </div>
      ))}
    </div>
  );
}

// ── Layer row ──────────────────────────────────────────────────────
function LayerRow({ id, label, color, on, onToggle, stratum, count }) {
  const [hov, setHov] = useState(false);
  return (
    <div
      onMouseEnter={() => setHov(true)}
      onMouseLeave={() => setHov(false)}
      style={{
        display: 'flex', alignItems: 'center', gap: 8,
        padding: '7px 10px', borderRadius: 4, cursor: 'pointer',
        background: hov ? 'rgba(120,180,255,0.04)' : 'transparent',
        border: '1px solid transparent',
        borderColor: hov ? 'rgba(120,180,255,0.1)' : 'transparent',
        transition: 'all 180ms',
      }}
      onClick={() => onToggle(id)}
    >
      {/* Visibility toggle */}
      <div style={{
        width: 14, height: 10, borderRadius: 2,
        background: on ? color : 'rgba(100,116,139,0.2)',
        border: `1px solid ${on ? color : 'rgba(100,116,139,0.3)'}`,
        boxShadow: on ? `0 0 6px ${color}88` : 'none',
        flexShrink: 0, transition: 'all 200ms',
      }}/>
      <div style={{ flex: 1 }}>
        <div style={{
          fontFamily: 'Cinzel, serif', fontSize: 10,
          letterSpacing: '0.15em', color: on ? '#e2e8f0' : '#475569',
          transition: 'color 200ms',
        }}>{label}</div>
        <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#334155', letterSpacing: '0.1em' }}>
          STR · {stratum} · {count} ENTITIES
        </div>
      </div>
      <div style={{
        width: 6, height: 6, borderRadius: '50%',
        background: on ? color : '#1e293b',
        boxShadow: on ? `0 0 6px ${color}` : 'none',
        transition: 'all 200ms',
      }}/>
    </div>
  );
}

// ── Layer Manager ──────────────────────────────────────────────────
function LayerManager({ layers, onToggleLayer }) {
  const layerDefs = [
    { id: 'prime',  label: 'PRIME NODES',   color: '#fbbf24', stratum: 0, count: 1 },
    { id: 'void',   label: 'VOID ZONES',    color: '#00e5ff', stratum: 1, count: 2 },
    { id: 'bio',    label: 'BIO ZONES',     color: '#39ff14', stratum: 2, count: 2 },
    { id: 'arcane', label: 'ARCANE NODES',  color: '#c084fc', stratum: 7, count: 1 },
    { id: 'forge',  label: 'FORGE NODES',   color: '#fbbf24', stratum: 5, count: 1 },
    { id: 'plasma', label: 'PLASMA ZONES',  color: '#f97316', stratum: 4, count: 1 },
    { id: 'routes', label: 'THREAD ROUTES', color: '#00e5ff', stratum: '-', count: 8 },
    { id: 'survey', label: 'SURVEY MARKS',  color: '#94a3b8', stratum: '-', count: 5 },
    { id: 'grid',   label: 'HEX GRID',      color: '#00e5ff', stratum: '-', count: '-' },
  ];

  return (
    <div style={{ padding: '14px 12px' }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 14 }}>
        <div style={{ fontFamily: 'Cinzel, serif', fontSize: 11, letterSpacing: '0.2em', color: '#fbbf24' }}>LAYER MATRIX</div>
        <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#475569' }}>9 STRATA</div>
      </div>

      {/* Stratum indicator */}
      <div style={{
        padding: '8px 10px', borderRadius: 4, marginBottom: 12,
        background: 'rgba(3,5,10,0.6)',
        border: '1px solid rgba(251,191,36,0.12)',
      }}>
        <div style={{ display: 'flex', gap: 2, marginBottom: 4 }}>
          {Array.from({length: 8}, (_, i) => (
            <div key={i} style={{
              flex: 1, height: 6, borderRadius: 1,
              background: i === 0 ? '#fbbf24' : i < 3 ? '#00e5ff' : i < 5 ? '#39ff14' : i < 6 ? '#f97316' : '#c084fc',
              opacity: 0.4 + (i === 0 ? 0.6 : 0),
            }}/>
          ))}
        </div>
        <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 7, color: '#475569' }}>
          STRATUM 0 → 7 · PRIME → SELENARCH
        </div>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {layerDefs.map(l => (
          <LayerRow key={l.id} {...l} on={layers[l.id] !== false} onToggle={onToggleLayer}/>
        ))}
      </div>

      {/* Blend mode */}
      <div style={{ marginTop: 14, padding: '8px 10px', borderRadius: 4, background: 'rgba(3,5,10,0.4)', border: '1px solid rgba(120,180,255,0.08)' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6 }}>
          <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#475569' }}>BLEND · COMPOSITE</span>
          <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#00e5ff' }}>SCREEN</span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#475569' }}>OPACITY</span>
          <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#fbbf24' }}>100%</span>
        </div>
      </div>
    </div>
  );
}

// ── Entity Inspector ───────────────────────────────────────────────
function EntityInspector({ regionId }) {
  const region = window.REGIONS ? window.REGIONS.find(r => r.id === regionId) : null;
  const [tab, setTab] = useState('info');
  const [t, setT] = useState(0);

  useEffect(() => {
    const id = setInterval(() => setT(x => x + 1), 50);
    return () => clearInterval(id);
  }, []);

  if (!region) {
    return (
      <div style={{ padding: '32px 16px', textAlign: 'center' }}>
        <div style={{ fontSize: 28, color: '#1e293b', marginBottom: 12 }}>◈</div>
        <div style={{ fontFamily: 'Cinzel, serif', fontSize: 11, letterSpacing: '0.2em', color: '#334155' }}>
          NO ENTITY BOUND
        </div>
        <div style={{ fontFamily: 'Cormorant Garamond, serif', fontStyle: 'italic', fontSize: 12, color: '#1e293b', marginTop: 8 }}>
          Select a region on the atlas to inspect its resonance.
        </div>
      </div>
    );
  }

  const pulse = 0.5 + 0.5 * Math.sin(t * 0.06);
  const instabs = [
    { label: 'RESONANCE',  value: (0.62 + pulse * 0.08).toFixed(3), color: region.color },
    { label: 'COHERENCE',  value: (0.91 - pulse * 0.04).toFixed(3), color: '#39ff14' },
    { label: 'DRIFT',      value: (0.02 + pulse * 0.01).toFixed(3), color: '#c084fc' },
    { label: 'ENTROPY',    value: (0.14 + pulse * 0.06).toFixed(3), color: '#f97316' },
  ];

  return (
    <div style={{ padding: '14px 12px' }}>
      {/* Entity header */}
      <div style={{ position: 'relative', marginBottom: 14 }}>
        <div style={{
          padding: '12px', borderRadius: 6,
          background: `linear-gradient(135deg, ${region.color}10, transparent)`,
          border: `1px solid ${region.color}33`,
          boxShadow: `0 0 20px ${region.glow}`,
        }}>
          {/* Corner marks */}
          <div style={{ position: 'absolute', top: 6, left: 6, width: 10, height: 10, borderTop: `1.5px solid ${region.color}88`, borderLeft: `1.5px solid ${region.color}88` }}/>
          <div style={{ position: 'absolute', bottom: 6, right: 6, width: 10, height: 10, borderBottom: `1.5px solid ${region.color}88`, borderRight: `1.5px solid ${region.color}88` }}/>

          {/* Sigil area */}
          <div style={{ textAlign: 'center', marginBottom: 8 }}>
            <svg width="60" height="60" viewBox="0 0 60 60">
              <circle cx="30" cy="30" r="26" fill="none" stroke={region.color} strokeWidth="0.8" strokeDasharray="2 3"
                style={{ transformOrigin: '30px 30px', animation: 'spin-slow 16s linear infinite' }}/>
              <circle cx="30" cy="30" r="18" fill="none" stroke={region.color} strokeWidth="0.8" strokeOpacity="0.5"/>
              <circle cx="30" cy="30" r="10" fill={region.color} fillOpacity="0.15" stroke={region.color} strokeWidth="1"/>
              <text x="30" y="35" textAnchor="middle" fontFamily="Cinzel, serif" fontSize="12" fill={region.color}>
                {region.type === 'prime-node' ? '⚜' : region.type === 'void-zone' ? '◈' : region.type === 'bio-zone' ? '⚚' : region.type === 'arcane-node' ? '𓂀' : region.type === 'forge-node' ? '✶' : '◎'}
              </text>
              <circle cx="30" cy="30" r={12 + pulse * 3} fill="none" stroke={region.color} strokeWidth="0.5"
                style={{ opacity: (1 - pulse) * 0.6 }}/>
            </svg>
          </div>

          <div style={{ fontFamily: 'Cinzel Decorative, serif', fontSize: 12, letterSpacing: '0.1em', color: region.color, textAlign: 'center', marginBottom: 2, textShadow: `0 0 12px ${region.glow}` }}>
            {region.name.toUpperCase()}
          </div>
          <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#475569', letterSpacing: '0.15em', textAlign: 'center' }}>
            {region.id.toUpperCase()} · STR.{region.stratum} · {region.hz}HZ
          </div>
        </div>
      </div>

      {/* Inspector tabs */}
      <div style={{ display: 'flex', gap: 2, marginBottom: 12 }}>
        {['info', 'data', 'lore'].map(tb => (
          <button key={tb} onClick={() => setTab(tb)} style={{
            flex: 1, padding: '5px 0',
            fontFamily: 'Cinzel, serif', fontSize: 8, letterSpacing: '0.15em',
            textTransform: 'uppercase',
            background: tab === tb ? `${region.color}18` : 'transparent',
            border: `1px solid ${tab === tb ? region.color : 'rgba(120,180,255,0.1)'}`,
            color: tab === tb ? region.color : '#475569',
            borderRadius: 2, cursor: 'pointer',
            transition: 'all 180ms',
          }}>{tb}</button>
        ))}
      </div>

      {tab === 'info' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          {/* Type chip */}
          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap', marginBottom: 4 }}>
            <Chip tone={region.type.includes('bio') ? 'bio' : region.type.includes('arcane') ? 'mythos' : region.type.includes('forge') || region.type.includes('prime') ? 'gold' : region.type.includes('plasma') ? 'ember' : 'quantum'}>
              {region.type.toUpperCase().replace('-', '·')}
            </Chip>
            <Chip tone="gold">STR·{region.stratum}</Chip>
            <Led tone="bio" on label="CHARTED"/>
          </div>

          {/* Live metrics */}
          {instabs.map(m => (
            <div key={m.label} style={{
              display: 'flex', justifyContent: 'space-between', alignItems: 'center',
              padding: '5px 8px', borderRadius: 3,
              background: 'rgba(3,5,10,0.5)',
              border: '1px solid rgba(120,180,255,0.06)',
            }}>
              <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#475569', letterSpacing: '0.12em' }}>{m.label}</span>
              <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 9, color: m.color, letterSpacing: '0.08em' }}>{m.value}</span>
            </div>
          ))}

          {/* Mini scope */}
          <MiniScope color={region.color} hz={region.hz} t={t}/>
        </div>
      )}

      {tab === 'data' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          {[
            ['COORDINATES',  `${region.x.toFixed(1)}°, ${region.y.toFixed(1)}°`],
            ['EXTENT',       `${region.rx}×${region.ry} QU`],
            ['STRATUM',      region.stratum],
            ['BASE HZ',      `${region.hz} HZ`],
            ['ENTITY TYPE',  region.type.toUpperCase()],
            ['SURVEY DATE',  'CYCLE·0xA4F2'],
            ['LINEAGE',      'PRIME → 0x3A'],
            ['CAPSULE ID',   `CAP/${region.id.toUpperCase()}/RT`],
          ].map(([k, v]) => (
            <div key={k} style={{
              display: 'flex', justifyContent: 'space-between',
              padding: '5px 8px', borderRadius: 3,
              background: 'rgba(3,5,10,0.5)',
              border: '1px solid rgba(120,180,255,0.06)',
            }}>
              <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#475569' }}>{k}</span>
              <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: region.color }}>{v}</span>
            </div>
          ))}
        </div>
      )}

      {tab === 'lore' && (
        <div>
          <div style={{ fontFamily: 'Cormorant Garamond, serif', fontStyle: 'italic', fontSize: 13, color: '#94a3b8', lineHeight: 1.65, padding: '8px 4px' }}>
            "{region.lore}"
          </div>
          <div style={{ marginTop: 12, padding: '8px 10px', borderRadius: 3, background: 'rgba(3,5,10,0.5)', border: `1px solid ${region.color}22` }}>
            <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 7, color: '#334155', marginBottom: 4 }}>ENTRY · QUANTUM CODEX</div>
            <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: region.color }}>
              {`NODE/${region.id.toUpperCase()}/RT · STATE: CHARTED · HZ: ${region.hz} · STR: ${region.stratum}`}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Mini oscilloscope ──────────────────────────────────────────────
function MiniScope({ color, hz, t }) {
  const W = 220, H = 44;
  const pts = Array.from({ length: 50 }, (_, i) => {
    const x = (i / 49) * W;
    const phase = (i / 49) * Math.PI * 5 + t * 0.07;
    const y = H / 2 + Math.sin(phase) * H * 0.3 + Math.sin(phase * 2.1) * 4;
    return `${x},${y}`;
  }).join(' ');
  return (
    <div style={{ background: '#0d1117', border: `1px solid ${color}33`, borderRadius: 3, padding: 4, marginTop: 4 }}>
      <svg width={W} height={H} viewBox={`0 0 ${W} ${H}`} style={{ display: 'block' }}>
        <polyline points={pts} fill="none" stroke={color} strokeWidth="1.2"
          style={{ filter: `drop-shadow(0 0 3px ${color})` }}/>
        <line x1={W/2} y1={0} x2={W/2} y2={H} stroke={color} strokeWidth="0.3" strokeOpacity="0.2" strokeDasharray="2 3"/>
      </svg>
      <div style={{ display: 'flex', justifyContent: 'space-between', padding: '2px 2px 0', fontFamily: 'JetBrains Mono, monospace', fontSize: 7, color: '#475569' }}>
        <span>SCOPE · LIVE</span>
        <span style={{ color }}>{hz}HZ</span>
      </div>
    </div>
  );
}

// ── World Codex ────────────────────────────────────────────────────
function WorldCodex() {
  const entries = [
    { title: 'VYRETH EXPANSE',   type: 'REGION',  color: '#00e5ff', detail: 'Probability-collapsed landmass. Stratum 3. Mapped: 62%.' },
    { title: 'THE PRIME SEAT',   type: 'NODE',    color: '#fbbf24', detail: 'Origin of all Lineage threads. Stratum 0. Immutable.' },
    { title: 'SELENARCH DRIFT',  type: 'CORRIDOR',color: '#c084fc', detail: 'Narrative-threaded corridor. Stratum 7. Active weave.' },
    { title: 'BRINE LATTICE',    type: 'BIOME',   color: '#39ff14', detail: 'Living crystalline grid. Cycle: 6h. Classified BIO-2.' },
    { title: 'AURIC THRESHOLD',  type: 'ZONE',    color: '#fbbf24', detail: 'Forge-resonant accumulation. Stratum 5. Order-claimed.' },
    { title: 'EMBER REACH',      type: 'ZONE',    color: '#f97316', detail: 'Plasma terminus. Stratum 4. Probe-access only.' },
    { title: 'NOX BASIN',        type: 'VOID',    color: '#00e5ff', detail: 'Deepest mapped stratum. Prime seal required.' },
    { title: 'SYLVAN MERIDIAN',  type: 'BIOME',   color: '#39ff14', detail: 'Bioluminescent forest-analog. Classification pending.' },
  ];

  const [search, setSearch] = useState('');
  const filtered = entries.filter(e =>
    e.title.toLowerCase().includes(search.toLowerCase()) ||
    e.type.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div style={{ padding: '14px 12px' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 12 }}>
        <div style={{ fontFamily: 'Cinzel, serif', fontSize: 11, letterSpacing: '0.2em', color: '#fbbf24' }}>WORLD CODEX</div>
        <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#475569' }}>{entries.length} ENTRIES</div>
      </div>

      {/* Search */}
      <div style={{ position: 'relative', marginBottom: 12 }}>
        <input
          value={search}
          onChange={e => setSearch(e.target.value)}
          placeholder="SEARCH CODEX..."
          style={{
            width: '100%', boxSizing: 'border-box',
            background: 'rgba(3,5,10,0.7)',
            border: '1px solid rgba(120,180,255,0.12)',
            borderRadius: 3, padding: '7px 10px',
            fontFamily: 'JetBrains Mono, monospace', fontSize: 9,
            color: '#94a3b8', letterSpacing: '0.12em',
            outline: 'none',
          }}
          onFocus={e => e.target.style.borderColor = 'rgba(0,229,255,0.4)'}
          onBlur={e => e.target.style.borderColor = 'rgba(120,180,255,0.12)'}
        />
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        {filtered.map(entry => (
          <div key={entry.title} style={{
            padding: '8px 10px', borderRadius: 4,
            background: 'rgba(3,5,10,0.5)',
            border: `1px solid ${entry.color}18`,
            cursor: 'pointer', transition: 'all 180ms',
          }}
            onMouseEnter={e => {
              e.currentTarget.style.borderColor = `${entry.color}44`;
              e.currentTarget.style.background = `${entry.color}08`;
              e.currentTarget.style.transform = 'translateX(2px)';
            }}
            onMouseLeave={e => {
              e.currentTarget.style.borderColor = `${entry.color}18`;
              e.currentTarget.style.background = 'rgba(3,5,10,0.5)';
              e.currentTarget.style.transform = 'translateX(0)';
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 3 }}>
              <span style={{ fontFamily: 'Cinzel, serif', fontSize: 9, letterSpacing: '0.15em', color: entry.color }}>{entry.title}</span>
              <span style={{
                fontFamily: 'JetBrains Mono, monospace', fontSize: 7, color: entry.color,
                padding: '1px 5px', borderRadius: 1, border: `1px solid ${entry.color}44`,
                background: `${entry.color}10`,
              }}>{entry.type}</span>
            </div>
            <div style={{ fontFamily: 'Space Grotesk, sans-serif', fontSize: 10, color: '#475569', lineHeight: 1.4 }}>
              {entry.detail}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ── Scribe Log ─────────────────────────────────────────────────────
function ScribeLog() {
  const [logs, setLogs] = useState([
    { ts: '00:14:07', entity: 'PRIME', color: '#fbbf24', msg: 'Prime Seat activated · Lineage thread open · stratum 0' },
    { ts: '00:14:22', entity: 'VYR',   color: '#00e5ff', msg: 'Vyreth Expanse surveyed · probability field stable · hz 432' },
    { ts: '00:15:03', entity: 'SEL',   color: '#c084fc', msg: 'Selenarch Drift · narrative thread bound · weave coherent' },
    { ts: '00:15:47', entity: 'BRI',   color: '#39ff14', msg: 'Brine Lattice · bloom cycle +2σ · 6h crystalline pulse' },
    { ts: '00:16:18', entity: 'AUR',   color: '#fbbf24', msg: 'Auric Threshold · forge-resonant · Order mandate filed' },
    { ts: '00:17:02', entity: 'SYS',   color: '#00e5ff', msg: 'Atlas integrity check · 8 regions charted · drift <0.02' },
    { ts: '00:17:33', entity: 'NOX',   color: '#f97316', msg: 'Nox Basin · probe coherence dropped · reschedule survey' },
    { ts: '00:18:01', entity: 'SYS',   color: '#39ff14', msg: 'Cartography session sealed · capsule committed · lineage preserved' },
  ]);

  const [autoRefresh, setAutoRefresh] = useState(true);

  useEffect(() => {
    if (!autoRefresh) return;
    const newEntries = [
      'Hex grid recalibrated · 140 tiles active · stratum confirmed',
      'Route thread VYR→SYL · drift 0.01 · coherent',
      'Survey point BSV·014 · bloom detected · classified BIO-2',
      'Prime Seat pulse · hz 432 · all nodes resonant',
      'Selenarch Drift · narrative density +14% · monitor',
    ];
    const id = setInterval(() => {
      const now = new Date();
      const ts = `${String(now.getHours()).padStart(2,'0')}:${String(now.getMinutes()).padStart(2,'0')}:${String(now.getSeconds()).padStart(2,'0')}`;
      const msg = newEntries[Math.floor(Math.random() * newEntries.length)];
      const colors = ['#00e5ff','#39ff14','#c084fc','#fbbf24'];
      const entities = ['SYS','ATLAS','QSV','PRIME'];
      setLogs(prev => [
        { ts, entity: entities[Math.floor(Math.random()*4)], color: colors[Math.floor(Math.random()*4)], msg },
        ...prev.slice(0, 14)
      ]);
    }, 3800);
    return () => clearInterval(id);
  }, [autoRefresh]);

  return (
    <div style={{ padding: '14px 12px' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 12 }}>
        <div style={{ fontFamily: 'Cinzel, serif', fontSize: 11, letterSpacing: '0.2em', color: '#fbbf24' }}>SCRIBE LOG</div>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <Led tone="bio" on={autoRefresh} label="LIVE"/>
          <button onClick={() => setAutoRefresh(p => !p)} style={{
            fontFamily: 'JetBrains Mono, monospace', fontSize: 7,
            padding: '3px 7px', borderRadius: 2,
            background: autoRefresh ? 'rgba(57,255,20,0.08)' : 'transparent',
            border: `1px solid ${autoRefresh ? '#39ff14' : 'rgba(120,180,255,0.15)'}`,
            color: autoRefresh ? '#39ff14' : '#475569',
            cursor: 'pointer',
          }}>{autoRefresh ? 'PAUSE' : 'RESUME'}</button>
        </div>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 3, maxHeight: 480, overflowY: 'auto' }}>
        {logs.map((log, i) => (
          <div key={i} style={{
            padding: '5px 8px', borderRadius: 3,
            background: 'rgba(3,5,10,0.5)',
            border: '1px solid rgba(120,180,255,0.05)',
            borderLeft: `2px solid ${log.color}55`,
            opacity: i === 0 ? 1 : Math.max(0.4, 1 - i * 0.05),
          }}>
            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 7, color: '#334155', flexShrink: 0 }}>{log.ts}</span>
              <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: log.color, letterSpacing: '0.1em', flexShrink: 0 }}>{log.entity}</span>
              <span style={{ fontFamily: 'Space Grotesk, sans-serif', fontSize: 9, color: '#64748b', lineHeight: 1.3 }}>{log.msg}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// Export
Object.assign(window, { ToolPalette, LayerManager, EntityInspector, WorldCodex, ScribeLog });
