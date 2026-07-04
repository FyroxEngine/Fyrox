/* Quantum Atlas — App Shell v2
   Lifts world state. Wires Atlas ↔ Forge ↔ Panels.
*/

const { useState, useEffect, useRef, useCallback } = React;

// ── Right Panel tabs ───────────────────────────────────────────────
const RIGHT_TABS = [
  { id: 'inspector', label: 'INSPECTOR', glyph: '◈' },
  { id: 'layers',    label: 'LAYERS',    glyph: '⬡' },
  { id: 'codex',     label: 'CODEX',     glyph: '𓂀' },
  { id: 'scribe',    label: 'SCRIBE',    glyph: '✦' },
];

// ── Status bar ─────────────────────────────────────────────────────
function StatusBar({ tool, zoom, view3D, regionId, regions, onZoomIn, onZoomOut, onToggle3D }) {
  const [time, setTime] = useState('');
  useEffect(() => {
    const tick = () => {
      const d = new Date();
      setTime(`${String(d.getHours()).padStart(2,'0')}:${String(d.getMinutes()).padStart(2,'0')}:${String(d.getSeconds()).padStart(2,'0')}`);
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, []);
  const region = regions && regionId ? regions.find(r => r.id === regionId) : null;

  return (
    <div style={{
      height: 28, flexShrink: 0,
      background: 'rgba(3,5,10,0.92)',
      borderTop: '1px solid rgba(120,180,255,0.07)',
      display: 'flex', alignItems: 'center',
      padding: '0 14px', gap: 16,
      fontFamily: 'JetBrains Mono, monospace', fontSize: 8,
      color: '#334155', letterSpacing: '0.12em',
    }}>
      <Led tone="bio" on label="ATLAS LIVE"/>
      <div style={{ color: '#475569' }}>TOOL · <span style={{ color: '#00e5ff' }}>{tool.toUpperCase()}</span></div>
      {region && <div style={{ color: '#475569' }}>SEL · <span style={{ color: region.color }}>{region.name.toUpperCase()}</span></div>}
      <div style={{ color: '#475569' }}>ZOOM · <span style={{ color: '#fbbf24' }}>{zoom}%</span></div>
      <div style={{ color: '#475569' }}>MODE · <span style={{ color: view3D ? '#c084fc' : '#00e5ff' }}>{view3D ? '3D·GLOBE' : '2D·FLAT'}</span></div>
      <div style={{ flex: 1 }}/>
      <div style={{ display: 'flex', gap: 4, alignItems: 'center' }}>
        {[{ label: '−', action: onZoomOut }, { label: '+', action: onZoomIn }].map(b => (
          <button key={b.label} onClick={b.action} style={{
            width: 18, height: 18, borderRadius: 2, background: 'transparent',
            border: '1px solid rgba(120,180,255,0.15)', color: '#64748b',
            cursor: 'pointer', fontSize: 11, display: 'flex', alignItems: 'center', justifyContent: 'center',
            transition: 'all 150ms',
          }}
            onMouseEnter={e => { e.currentTarget.style.borderColor = '#00e5ff44'; e.currentTarget.style.color = '#00e5ff'; }}
            onMouseLeave={e => { e.currentTarget.style.borderColor = 'rgba(120,180,255,0.15)'; e.currentTarget.style.color = '#64748b'; }}
          >{b.label}</button>
        ))}
        <button onClick={onToggle3D} style={{
          padding: '0 8px', height: 18, borderRadius: 2,
          background: view3D ? 'rgba(192,132,252,0.12)' : 'rgba(0,229,255,0.08)',
          border: `1px solid ${view3D ? 'rgba(192,132,252,0.4)' : 'rgba(0,229,255,0.3)'}`,
          color: view3D ? '#c084fc' : '#00e5ff',
          fontFamily: 'JetBrains Mono, monospace', fontSize: 7,
          letterSpacing: '0.12em', cursor: 'pointer', transition: 'all 200ms',
        }}>{view3D ? '3D' : '2D'}</button>
      </div>
      <div style={{ color: '#1e293b' }}>CYCLE·{time}</div>
      <div style={{ width: 6, height: 6, borderRadius: '50%', background: '#39ff14',
        boxShadow: '0 0 6px #39ff14', animation: 'led-blink 1.4s ease-in-out infinite' }}/>
    </div>
  );
}

// ── Right panel ────────────────────────────────────────────────────
function RightPanel({ activeTab, onTabChange, selectedRegion, regions, layers, onToggleLayer }) {
  // Keep REGIONS global in sync
  useEffect(() => { window.REGIONS = regions; }, [regions]);

  return (
    <div style={{
      width: 260, flexShrink: 0,
      background: 'rgba(7,9,15,0.92)',
      backdropFilter: 'blur(12px)',
      borderLeft: '1px solid rgba(120,180,255,0.08)',
      display: 'flex', flexDirection: 'column',
      overflow: 'hidden',
    }}>
      <div style={{ display: 'flex', borderBottom: '1px solid rgba(120,180,255,0.08)', flexShrink: 0 }}>
        {RIGHT_TABS.map(tab => {
          const active = activeTab === tab.id;
          return (
            <button key={tab.id} onClick={() => onTabChange(tab.id)} style={{
              flex: 1, padding: '10px 0',
              background: active ? 'rgba(251,191,36,0.06)' : 'transparent',
              border: 'none',
              borderBottom: `2px solid ${active ? '#fbbf24' : 'transparent'}`,
              color: active ? '#fbbf24' : '#334155',
              cursor: 'pointer', transition: 'all 180ms',
              display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2,
            }}
              onMouseEnter={e => { if (!active) e.currentTarget.style.color = '#64748b'; }}
              onMouseLeave={e => { if (!active) e.currentTarget.style.color = '#334155'; }}
            >
              <span style={{ fontSize: 11 }}>{tab.glyph}</span>
              <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 6, letterSpacing: '0.15em' }}>{tab.label}</span>
            </button>
          );
        })}
      </div>
      <div style={{ flex: 1, overflowY: 'auto', overflowX: 'hidden' }}>
        {activeTab === 'inspector' && <EntityInspector regionId={selectedRegion}/>}
        {activeTab === 'layers'    && <LayerManager layers={layers} onToggleLayer={onToggleLayer}/>}
        {activeTab === 'codex'     && <WorldCodex/>}
        {activeTab === 'scribe'    && <ScribeLog/>}
      </div>
    </div>
  );
}

// ── Top Nav ────────────────────────────────────────────────────────
function TopNav({ appScreen, onScreenChange, projectName, onProjectName, world }) {
  const [editing, setEditing] = useState(false);
  const [nameVal, setNameVal] = useState(projectName);
  useEffect(() => { setNameVal(projectName); }, [projectName]);

  const NAV_ITEMS = [
    { id: 'atlas',  label: 'ATLAS',  color: '#00e5ff' },
    { id: 'forge',  label: 'FORGE',  color: '#fbbf24' },
    { id: 'codex',  label: 'CODEX',  color: '#c084fc' },
    { id: 'export', label: 'EXPORT', color: '#39ff14' },
  ];

  return (
    <nav style={{
      height: 46, flexShrink: 0,
      background: 'rgba(3,5,10,0.96)',
      backdropFilter: 'blur(20px)',
      borderBottom: '1px solid rgba(251,191,36,0.14)',
      display: 'flex', alignItems: 'center',
      padding: '0 16px', gap: 16, zIndex: 10,
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, flexShrink: 0 }}>
        <div style={{
          width: 28, height: 28, background: 'rgba(251,191,36,0.1)',
          border: '1px solid rgba(251,191,36,0.3)', borderRadius: 4,
          display: 'flex', alignItems: 'center', justifyContent: 'center',
        }}>
          <span style={{ fontFamily: 'Cinzel Decorative, serif', fontSize: 13, color: '#fbbf24' }}>Q</span>
        </div>
        <div>
          <div style={{ fontFamily: 'Cinzel Decorative, serif', fontSize: 11, letterSpacing: '0.18em', color: '#e2e8f0' }}>QUANTUM ATLAS</div>
          <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 7, color: '#fbbf24', letterSpacing: '0.25em', opacity: 0.7 }}>BIOSPARK · CARTOGRAPHY</div>
        </div>
      </div>

      <div style={{ width: 1, height: 24, background: 'rgba(251,191,36,0.15)', flexShrink: 0 }}/>

      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        {editing ? (
          <input autoFocus value={nameVal}
            onChange={e => setNameVal(e.target.value)}
            onBlur={() => { setEditing(false); onProjectName(nameVal); }}
            onKeyDown={e => { if (e.key === 'Enter') { setEditing(false); onProjectName(nameVal); } }}
            style={{
              background: 'rgba(3,5,10,0.8)',
              border: '1px solid rgba(0,229,255,0.4)', borderRadius: 2,
              padding: '3px 8px', fontFamily: 'Cinzel, serif', fontSize: 10,
              letterSpacing: '0.15em', color: '#00e5ff', outline: 'none', width: 220,
            }}
          />
        ) : (
          <div onClick={() => setEditing(true)} style={{
            fontFamily: 'Cinzel, serif', fontSize: 10, letterSpacing: '0.15em',
            color: '#94a3b8', cursor: 'text', padding: '3px 6px', borderRadius: 2,
            border: '1px solid transparent', transition: 'all 180ms',
          }}
            onMouseEnter={e => { e.currentTarget.style.borderColor = 'rgba(0,229,255,0.2)'; e.currentTarget.style.color = '#e2e8f0'; }}
            onMouseLeave={e => { e.currentTarget.style.borderColor = 'transparent'; e.currentTarget.style.color = '#94a3b8'; }}
          >{nameVal}</div>
        )}
        <Chip tone="gold">v1.0</Chip>
      </div>

      <div style={{ flex: 1 }}/>

      <div style={{ display: 'flex', gap: 2, alignItems: 'center' }}>
        {NAV_ITEMS.map(btn => {
          const active = appScreen === btn.id;
          return (
            <button key={btn.id} onClick={() => onScreenChange(btn.id)} style={{
              fontFamily: 'Cinzel, serif', fontSize: 9, letterSpacing: '0.18em',
              padding: '5px 14px',
              background: active ? `${btn.color}14` : 'transparent',
              border: `1px solid ${active ? btn.color : 'rgba(120,180,255,0.1)'}`,
              color: active ? btn.color : '#475569', borderRadius: 2, cursor: 'pointer',
              boxShadow: active ? `0 0 10px ${btn.color}33` : 'none',
              transition: 'all 180ms',
            }}
              onMouseEnter={e => { if (!active) { e.currentTarget.style.borderColor = `${btn.color}44`; e.currentTarget.style.color = btn.color; }}}
              onMouseLeave={e => { if (!active) { e.currentTarget.style.borderColor = 'rgba(120,180,255,0.1)'; e.currentTarget.style.color = '#475569'; }}}
            >{btn.label}</button>
          );
        })}
      </div>

      <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexShrink: 0 }}>
        <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#475569' }}>
          {world.regions.length}R · {world.routes.length}T · {world.survey.length}S
        </span>
        <Led tone="quantum" on label="UPLINK"/>
        <Chip tone="bio">RT · LIVE</Chip>
      </div>
    </nav>
  );
}

// ── Map toolbar ────────────────────────────────────────────────────
function MapToolbar({ view3D, onToggle3D, zoom, onZoomIn, onZoomOut, onZoomReset }) {
  return (
    <div style={{ position: 'absolute', top: 14, right: 14, display: 'flex', flexDirection: 'column', gap: 4, zIndex: 5 }}>
      <div style={{
        background: 'rgba(3,5,10,0.88)', backdropFilter: 'blur(8px)',
        border: '1px solid rgba(120,180,255,0.12)', borderRadius: 4, padding: '4px',
        display: 'flex', flexDirection: 'column', gap: 3,
      }}>
        {[
          { label: '2D', active: !view3D, onClick: () => { if (view3D) onToggle3D(); }, color: '#00e5ff' },
          { label: '3D', active: view3D,  onClick: () => { if (!view3D) onToggle3D(); }, color: '#c084fc' },
        ].map(btn => (
          <button key={btn.label} onClick={btn.onClick} style={{
            width: 30, height: 22, borderRadius: 2,
            background: btn.active ? `${btn.color}18` : 'transparent',
            border: `1px solid ${btn.active ? btn.color : 'transparent'}`,
            color: btn.active ? btn.color : '#475569',
            fontFamily: 'JetBrains Mono, monospace', fontSize: 8,
            letterSpacing: '0.1em', cursor: 'pointer', transition: 'all 180ms',
          }}>{btn.label}</button>
        ))}
      </div>
      <div style={{
        background: 'rgba(3,5,10,0.88)', backdropFilter: 'blur(8px)',
        border: '1px solid rgba(120,180,255,0.12)', borderRadius: 4, padding: '4px',
        display: 'flex', flexDirection: 'column', gap: 3, alignItems: 'center',
      }}>
        {[{ label: '+', onClick: onZoomIn }, { label: '⊙', onClick: onZoomReset }, { label: '−', onClick: onZoomOut }].map(btn => (
          <button key={btn.label} onClick={btn.onClick} style={{
            width: 30, height: 22, borderRadius: 2,
            background: 'transparent', border: '1px solid rgba(120,180,255,0.1)',
            color: '#64748b', cursor: 'pointer', fontSize: 12, transition: 'all 150ms',
          }}
            onMouseEnter={e => { e.currentTarget.style.borderColor = '#00e5ff44'; e.currentTarget.style.color = '#00e5ff'; }}
            onMouseLeave={e => { e.currentTarget.style.borderColor = 'rgba(120,180,255,0.1)'; e.currentTarget.style.color = '#64748b'; }}
          >{btn.label}</button>
        ))}
      </div>
    </div>
  );
}

// ── Export Screen ──────────────────────────────────────────────────
function ExportScreen({ world }) {
  const [format, setFormat] = useState('json');
  const [copied, setCopied] = useState(false);

  const exportData = format === 'json'
    ? JSON.stringify(world, null, 2)
    : [
        '# QUANTUM ATLAS · WORLD EXPORT',
        `# ${new Date().toISOString()}`,
        '',
        '## REGIONS',
        ...world.regions.map(r => `${r.id} | ${r.name} | ${r.type} | ${r.x},${r.y} | ${r.hz}hz | str:${r.stratum}`),
        '',
        '## ROUTES',
        ...world.routes.map(r => `${r.id} | ${r.label || r.type} | ${r.from} → ${r.to} | ${r.hz}hz`),
        '',
        '## SURVEY',
        ...world.survey.map(s => `${s.id} | ${s.label} | ${s.x},${s.y} | ${s.note}`),
      ].join('\n');

  const handleCopy = () => {
    navigator.clipboard.writeText(exportData).then(() => { setCopied(true); setTimeout(() => setCopied(false), 1800); });
  };

  return (
    <div style={{ flex: 1, overflowY: 'auto', padding: '32px' }}>
      <div style={{ maxWidth: 860, margin: '0 auto' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 24 }}>
          <div style={{ fontFamily: 'Cinzel Decorative, serif', fontSize: 18, letterSpacing: '0.12em', color: '#39ff14', textShadow: '0 0 24px rgba(57,255,20,0.3)' }}>
            EXPORT · SEAL
          </div>
          <div style={{ height: 1, flex: 1, background: 'linear-gradient(90deg, rgba(57,255,20,0.3), transparent)' }}/>
        </div>
        <div style={{ fontFamily: 'Cormorant Garamond, serif', fontStyle: 'italic', fontSize: 14, color: '#475569', marginBottom: 28 }}>
          "Every Capsule a Lineage. Seal the world, preserve the thread."
        </div>

        {/* Stats */}
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 16, marginBottom: 28 }}>
          {[
            { label: 'REGIONS', value: world.regions.length, color: '#00e5ff' },
            { label: 'ROUTES',  value: world.routes.length,  color: '#c084fc' },
            { label: 'SURVEYS', value: world.survey.length,  color: '#39ff14' },
          ].map(s => (
            <div key={s.label} style={{
              padding: '18px 20px', borderRadius: 6,
              background: `${s.color}0a`,
              border: `1px solid ${s.color}22`,
              position: 'relative', overflow: 'hidden',
            }}>
              <div style={{ position: 'absolute', top: 4, left: 4, width: 10, height: 10, borderTop: `1.5px solid ${s.color}66`, borderLeft: `1.5px solid ${s.color}66` }}/>
              <div style={{ fontFamily: 'Cinzel Decorative, serif', fontSize: 32, color: s.color, lineHeight: 1 }}>{s.value}</div>
              <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 9, color: '#475569', letterSpacing: '0.2em', marginTop: 4 }}>{s.label} CHARTED</div>
            </div>
          ))}
        </div>

        {/* Format picker */}
        <div style={{ display: 'flex', gap: 8, marginBottom: 16 }}>
          {[{ id: 'json', label: 'JSON · STRUCTURED' }, { id: 'text', label: 'TEXT · CODEX' }].map(f => (
            <button key={f.id} onClick={() => setFormat(f.id)} style={{
              fontFamily: 'JetBrains Mono, monospace', fontSize: 9, letterSpacing: '0.15em',
              padding: '6px 14px', borderRadius: 3,
              background: format === f.id ? 'rgba(57,255,20,0.12)' : 'transparent',
              border: `1px solid ${format === f.id ? '#39ff14' : 'rgba(120,180,255,0.12)'}`,
              color: format === f.id ? '#39ff14' : '#475569',
              cursor: 'pointer', transition: 'all 150ms',
            }}>{f.label}</button>
          ))}
          <button onClick={handleCopy} style={{
            marginLeft: 'auto',
            fontFamily: 'Cinzel, serif', fontSize: 9, letterSpacing: '0.18em',
            padding: '6px 18px', borderRadius: 3,
            background: copied ? 'rgba(57,255,20,0.18)' : 'rgba(57,255,20,0.1)',
            border: `1px solid ${copied ? '#39ff14' : 'rgba(57,255,20,0.3)'}`,
            color: copied ? '#39ff14' : '#64748b',
            cursor: 'pointer', transition: 'all 200ms',
          }}>{copied ? '✓ SEALED' : '⎘ COPY SEAL'}</button>
        </div>

        <pre style={{
          background: 'rgba(3,5,10,0.8)',
          border: '1px solid rgba(120,180,255,0.1)',
          borderRadius: 4, padding: '16px 18px',
          fontFamily: 'JetBrains Mono, monospace', fontSize: 10,
          color: '#64748b', lineHeight: 1.7,
          maxHeight: 400, overflowY: 'auto',
          whiteSpace: 'pre-wrap', wordBreak: 'break-word',
        }}>{exportData}</pre>
      </div>
    </div>
  );
}

// ── Codex Screen (full-page) ────────────────────────────────────────
function CodexScreen({ world }) {
  return (
    <div style={{ flex: 1, overflowY: 'auto', padding: '32px' }}>
      <div style={{ maxWidth: 1100, margin: '0 auto' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 8 }}>
          <div style={{ fontFamily: 'Cinzel Decorative, serif', fontSize: 18, letterSpacing: '0.12em', color: '#c084fc', textShadow: '0 0 24px rgba(192,132,252,0.3)' }}>
            WORLD CODEX
          </div>
          <div style={{ height: 1, flex: 1, background: 'linear-gradient(90deg, rgba(192,132,252,0.3), transparent)' }}/>
          <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#475569' }}>{world.regions.length} ENTRIES</div>
        </div>
        <div style={{ fontFamily: 'Cormorant Garamond, serif', fontStyle: 'italic', fontSize: 14, color: '#475569', marginBottom: 28 }}>
          "Every region a lineage. Every lineage, a tell."
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(300px, 1fr))', gap: 16 }}>
          {world.regions.map(r => (
            <div key={r.id} style={{
              padding: '18px 20px', borderRadius: 6,
              background: `linear-gradient(135deg, ${r.color}08, transparent)`,
              border: `1px solid ${r.color}22`,
              position: 'relative', transition: 'all 220ms',
            }}
              onMouseEnter={e => { e.currentTarget.style.borderColor = `${r.color}55`; e.currentTarget.style.transform = 'translateY(-2px)'; }}
              onMouseLeave={e => { e.currentTarget.style.borderColor = `${r.color}22`; e.currentTarget.style.transform = 'translateY(0)'; }}
            >
              <div style={{ position: 'absolute', top: 6, left: 6, width: 10, height: 10, borderTop: `1.5px solid ${r.color}55`, borderLeft: `1.5px solid ${r.color}55` }}/>
              <div style={{ position: 'absolute', bottom: 6, right: 6, width: 10, height: 10, borderBottom: `1.5px solid ${r.color}55`, borderRight: `1.5px solid ${r.color}55` }}/>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 8 }}>
                <div style={{ fontFamily: 'Cinzel, serif', fontSize: 11, letterSpacing: '0.18em', color: r.color }}>{r.name.toUpperCase()}</div>
                <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 7, padding: '2px 6px', borderRadius: 1, border: `1px solid ${r.color}44`, color: r.color, background: `${r.color}10`, whiteSpace: 'nowrap', marginLeft: 8 }}>
                  {r.type.replace('-', '·').toUpperCase()}
                </div>
              </div>
              <div style={{ display: 'flex', gap: 6, marginBottom: 10, flexWrap: 'wrap' }}>
                <Chip tone={r.type.includes('bio') ? 'bio' : r.type.includes('arcane') ? 'mythos' : r.type.includes('prime') || r.type.includes('forge') ? 'gold' : r.type.includes('plasma') ? 'ember' : 'quantum'}>
                  STR·{r.stratum}
                </Chip>
                <Chip tone="gold">{r.hz}HZ</Chip>
                <Chip tone="quantum">{r.x}°, {r.y}°</Chip>
              </div>
              <div style={{ fontFamily: 'Cormorant Garamond, serif', fontStyle: 'italic', fontSize: 13, color: '#64748b', lineHeight: 1.55 }}>
                {r.lore || 'No cartographic note recorded.'}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// ── Atlas App ──────────────────────────────────────────────────────
function AtlasApp() {
  // World state — single source of truth
  const [world, setWorld] = useState(() => window.loadWorld ? window.loadWorld() : {
    regions: window.DEFAULT_REGIONS || [],
    routes:  window.DEFAULT_ROUTES  || [],
    survey:  window.DEFAULT_SURVEY  || [],
  });

  // Keep global REGIONS alias in sync for legacy components (EntityInspector etc.)
  useEffect(() => { window.REGIONS = world.regions; }, [world.regions]);

  // Persist world
  useEffect(() => { if (window.saveWorld) window.saveWorld(world); }, [world]);

  // UI state
  const [appScreen, setAppScreen]       = useState(() => localStorage.getItem('atlas-screen') || 'atlas');
  const [activeTool, setActiveTool]     = useState(() => localStorage.getItem('atlas-tool') || 'select');
  const [selectedRegion, setSelectedRegion] = useState(() => localStorage.getItem('atlas-region') || null);
  const [rightTab, setRightTab]         = useState(() => localStorage.getItem('atlas-right-tab') || 'inspector');
  const [view3D, setView3D]             = useState(() => localStorage.getItem('atlas-3d') === 'true');
  const [zoom, setZoom]                 = useState(() => parseInt(localStorage.getItem('atlas-zoom') || '100'));
  const [projectName, setProjectName]   = useState(() => localStorage.getItem('atlas-project') || 'PRIME CONSTELLATION · WORLD 01');
  const [layers, setLayers]             = useState(() => {
    try { return JSON.parse(localStorage.getItem('atlas-layers') || '{}'); } catch { return {}; }
  });

  useEffect(() => { localStorage.setItem('atlas-screen',    appScreen);        }, [appScreen]);
  useEffect(() => { localStorage.setItem('atlas-tool',      activeTool);       }, [activeTool]);
  useEffect(() => { localStorage.setItem('atlas-region',    selectedRegion || ''); }, [selectedRegion]);
  useEffect(() => { localStorage.setItem('atlas-right-tab', rightTab);         }, [rightTab]);
  useEffect(() => { localStorage.setItem('atlas-3d',        String(view3D));   }, [view3D]);
  useEffect(() => { localStorage.setItem('atlas-zoom',      String(zoom));     }, [zoom]);
  useEffect(() => { localStorage.setItem('atlas-project',   projectName);      }, [projectName]);
  useEffect(() => { localStorage.setItem('atlas-layers',    JSON.stringify(layers)); }, [layers]);

  // Keyboard shortcuts (atlas screen only)
  useEffect(() => {
    const handler = (e) => {
      if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
      const tool = ['v','r','l','a','s','e','m','b'];
      const ids  = ['select','region','route','annotate','survey','erase','measure','lore'];
      const idx  = tool.indexOf(e.key.toLowerCase());
      if (idx >= 0) setActiveTool(ids[idx]);
      if (e.key === 'Escape') setSelectedRegion(null);
      if (e.key === 'F' || e.key === 'f') setAppScreen('forge');
      if (e.key === '1') setAppScreen('atlas');
      if (e.key === '2') setAppScreen('forge');
      if (e.key === '3') setAppScreen('codex');
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  const handleSelectRegion = useCallback((id) => {
    setSelectedRegion(prev => prev === id ? null : id);
    setRightTab('inspector');
  }, []);

  const toggleLayer = useCallback((id) => {
    setLayers(prev => ({ ...prev, [id]: prev[id] === false ? true : false }));
  }, []);

  const layerActive = (id) => layers[id] !== false;
  const layerMap = {
    void:   layerActive('void'),   arcane: layerActive('arcane'),
    bio:    layerActive('bio'),    forge:  layerActive('forge'),
    plasma: layerActive('plasma'), prime:  layerActive('prime'),
    routes: layerActive('routes'), survey: layerActive('survey'),
    grid:   layerActive('grid'),
  };

  const isAtlas = appScreen === 'atlas';

  return (
    <div style={{
      width: '100vw', height: '100vh',
      display: 'flex', flexDirection: 'column',
      background: '#03050a',
      backgroundImage: 'radial-gradient(ellipse 900px 500px at 10% 20%, rgba(20,40,100,0.3) 0%, transparent 70%), radial-gradient(ellipse 700px 400px at 85% 60%, rgba(10,30,80,0.25) 0%, transparent 70%)',
      overflow: 'hidden',
      fontFamily: 'Space Grotesk, sans-serif',
    }}>
      <TopNav
        appScreen={appScreen} onScreenChange={setAppScreen}
        projectName={projectName} onProjectName={setProjectName}
        world={world}
      />

      <div style={{ flex: 1, display: 'flex', overflow: 'hidden', minHeight: 0 }}>
        {/* Tool palette — only on atlas screen */}
        {isAtlas && <ToolPalette activeTool={activeTool} onSelectTool={setActiveTool}/>}

        {/* Main content area */}
        <div style={{ flex: 1, display: 'flex', overflow: 'hidden', position: 'relative' }}>
          {/* ATLAS screen */}
          {isAtlas && (
            <>
              <div style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
                <AtlasMapView
                  selectedId={selectedRegion}
                  onSelect={handleSelectRegion}
                  layers={layerMap}
                  view3D={view3D}
                  zoom={zoom}
                  regions={world.regions}
                  routes={world.routes}
                  survey={world.survey}
                />
                <MapToolbar
                  view3D={view3D} onToggle3D={() => setView3D(v => !v)}
                  zoom={zoom}
                  onZoomIn={() => setZoom(z => Math.min(400, z + 25))}
                  onZoomOut={() => setZoom(z => Math.max(25, z - 25))}
                  onZoomReset={() => setZoom(100)}
                />
                {/* Selected region quick-info */}
                {selectedRegion && (() => {
                  const r = world.regions.find(x => x.id === selectedRegion);
                  if (!r) return null;
                  return (
                    <div style={{
                      position: 'absolute', bottom: 20, left: '50%',
                      transform: 'translateX(-50%)',
                      background: 'rgba(3,5,10,0.9)', backdropFilter: 'blur(10px)',
                      border: `1px solid ${r.color}44`, borderRadius: 4,
                      padding: '8px 16px', display: 'flex', gap: 16, alignItems: 'center',
                      boxShadow: `0 0 20px ${r.glow}`, pointerEvents: 'none',
                      zIndex: 4,
                    }}>
                      <div style={{ width: 8, height: 8, borderRadius: '50%', background: r.color, boxShadow: `0 0 8px ${r.color}` }}/>
                      <div style={{ fontFamily: 'Cinzel, serif', fontSize: 10, letterSpacing: '0.18em', color: r.color }}>{r.name.toUpperCase()}</div>
                      <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#64748b' }}>STR·{r.stratum} · {r.hz}HZ</div>
                      <button onClick={() => { setSelectedRegion(null); setAppScreen('forge'); }}
                        style={{
                          fontFamily: 'JetBrains Mono, monospace', fontSize: 8,
                          padding: '3px 8px', borderRadius: 2, cursor: 'pointer',
                          background: `${r.color}18`, border: `1px solid ${r.color}55`,
                          color: r.color, pointerEvents: 'all',
                        }}>FORGE ⟿</button>
                    </div>
                  );
                })()}
              </div>
              <RightPanel
                activeTab={rightTab} onTabChange={setRightTab}
                selectedRegion={selectedRegion}
                regions={world.regions}
                layers={layerMap} onToggleLayer={toggleLayer}
              />
            </>
          )}

          {/* FORGE screen */}
          {appScreen === 'forge' && (
            <ForgeScreen world={world} onWorldChange={setWorld}/>
          )}

          {/* CODEX screen */}
          {appScreen === 'codex' && (
            <CodexScreen world={world}/>
          )}

          {/* EXPORT screen */}
          {appScreen === 'export' && (
            <ExportScreen world={world}/>
          )}
        </div>
      </div>

      <StatusBar
        tool={activeTool} zoom={zoom} view3D={view3D}
        regionId={selectedRegion} regions={world.regions}
        onZoomIn={() => setZoom(z => Math.min(400, z + 25))}
        onZoomOut={() => setZoom(z => Math.max(25, z - 25))}
        onToggle3D={() => setView3D(v => !v)}
      />

      <AtlasTweaks/>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<AtlasApp/>);
