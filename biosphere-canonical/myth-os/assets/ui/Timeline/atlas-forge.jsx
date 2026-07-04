/* Quantum Atlas — Forge Screen
   Three sub-forges: Region · Route · Survey
   Reads/writes world state via props (regions, routes, survey + setters).
*/

// ── Shared forge styles ────────────────────────────────────────────
const FORGE_INPUT_STYLE = {
  width: '100%', boxSizing: 'border-box',
  background: 'rgba(3,5,10,0.7)',
  border: '1px solid rgba(120,180,255,0.14)',
  borderRadius: 3, padding: '8px 10px',
  fontFamily: 'Space Grotesk, sans-serif', fontSize: 13,
  color: '#e2e8f0', outline: 'none',
  transition: 'border-color 180ms',
};
const FORGE_LABEL_STYLE = {
  fontFamily: 'JetBrains Mono, monospace', fontSize: 9,
  letterSpacing: '0.2em', color: '#64748b',
  marginBottom: 5, display: 'block',
};
const FORGE_BTN = (color, glow) => ({
  fontFamily: 'Cinzel, serif', fontSize: 11,
  letterSpacing: '0.2em', padding: '10px 24px',
  background: `${color}18`,
  border: `1px solid ${color}`,
  color, borderRadius: 3, cursor: 'pointer',
  boxShadow: `0 0 14px ${glow}`,
  transition: 'all 200ms',
});

function ForgeInput({ label, value, onChange, placeholder, type = 'text' }) {
  const [focused, setFocused] = React.useState(false);
  return (
    <div style={{ marginBottom: 14 }}>
      <span style={FORGE_LABEL_STYLE}>{label}</span>
      <input
        type={type}
        value={value}
        onChange={e => onChange(e.target.value)}
        placeholder={placeholder}
        onFocus={e => { setFocused(true); e.target.style.borderColor = 'rgba(0,229,255,0.45)'; }}
        onBlur={e => { setFocused(false); e.target.style.borderColor = 'rgba(120,180,255,0.14)'; }}
        style={{ ...FORGE_INPUT_STYLE, borderColor: focused ? 'rgba(0,229,255,0.45)' : 'rgba(120,180,255,0.14)' }}
      />
    </div>
  );
}

function ForgeTextarea({ label, value, onChange, placeholder, rows = 3 }) {
  return (
    <div style={{ marginBottom: 14 }}>
      <span style={FORGE_LABEL_STYLE}>{label}</span>
      <textarea
        value={value}
        onChange={e => onChange(e.target.value)}
        placeholder={placeholder}
        rows={rows}
        onFocus={e => e.target.style.borderColor = 'rgba(0,229,255,0.45)'}
        onBlur={e => e.target.style.borderColor = 'rgba(120,180,255,0.14)'}
        style={{ ...FORGE_INPUT_STYLE, resize: 'vertical', lineHeight: 1.5 }}
      />
    </div>
  );
}

function ForgeSelect({ label, value, onChange, options }) {
  return (
    <div style={{ marginBottom: 14 }}>
      <span style={FORGE_LABEL_STYLE}>{label}</span>
      <select
        value={value}
        onChange={e => onChange(e.target.value)}
        style={{ ...FORGE_INPUT_STYLE, cursor: 'pointer' }}
      >
        {options.map(o => (
          <option key={o.value} value={o.value} style={{ background: '#0d1117' }}>{o.label}</option>
        ))}
      </select>
    </div>
  );
}

function ForgeSlider({ label, value, onChange, min, max, step = 1, unit = '' }) {
  return (
    <div style={{ marginBottom: 14 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 5 }}>
        <span style={FORGE_LABEL_STYLE}>{label}</span>
        <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 9, color: '#00e5ff' }}>{value}{unit}</span>
      </div>
      <input
        type="range" min={min} max={max} step={step} value={value}
        onChange={e => onChange(Number(e.target.value))}
        style={{ width: '100%', accentColor: '#00e5ff', cursor: 'pointer', height: 4 }}
      />
      <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 3 }}>
        <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 7, color: '#334155' }}>{min}{unit}</span>
        <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 7, color: '#334155' }}>{max}{unit}</span>
      </div>
    </div>
  );
}

// ── Region preview sigil ───────────────────────────────────────────
function RegionPreview({ type, name, hz, rx, ry }) {
  const def = window.TYPE_DEFS?.[type] || { color: '#00e5ff', glow: 'rgba(0,229,255,0.5)' };
  const { color, glow } = def;
  const [t, setT] = React.useState(0);
  React.useEffect(() => {
    const id = setInterval(() => setT(x => x + 1), 50);
    return () => clearInterval(id);
  }, []);
  const pulse = 0.5 + 0.5 * Math.sin(t * 0.06);

  return (
    <div style={{
      display: 'flex', flexDirection: 'column', alignItems: 'center',
      padding: '24px 16px',
      background: `radial-gradient(ellipse at center, ${color}12 0%, transparent 70%)`,
      border: `1px solid ${color}22`,
      borderRadius: 6,
    }}>
      <svg width="140" height="100" viewBox="0 0 140 100">
        <defs>
          <radialGradient id="prevGrad" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stopColor={color} stopOpacity="0.3"/>
            <stop offset="100%" stopColor={color} stopOpacity="0"/>
          </radialGradient>
        </defs>
        {/* Outer glow */}
        <ellipse cx="70" cy="50" rx={Math.min(60, rx * 5 + 10)} ry={Math.min(40, ry * 5 + 8)}
          fill="url(#prevGrad)" opacity={0.5 + 0.2 * pulse}/>
        {/* Region body */}
        <ellipse cx="70" cy="50" rx={Math.min(45, rx * 4)} ry={Math.min(32, ry * 4)}
          fill={color} fillOpacity="0.12"
          stroke={color} strokeWidth="1.5" strokeOpacity="0.8"
          style={{ filter: `drop-shadow(0 0 6px ${color})` }}
        />
        {/* Center dot */}
        <circle cx="70" cy="50" r="5" fill={color} opacity={0.8 + 0.2 * pulse}/>
        {/* Pulse ring */}
        <circle cx="70" cy="50" r={8 + pulse * 5}
          fill="none" stroke={color} strokeWidth="0.8" opacity={(1 - pulse) * 0.7}/>
        {/* Hz label */}
        <text x="70" y="88" textAnchor="middle"
          fontFamily="JetBrains Mono, monospace" fontSize="8" fill={color} opacity="0.6" letterSpacing="1">
          {hz}HZ
        </text>
      </svg>
      <div style={{ fontFamily: 'Cinzel, serif', fontSize: 11, letterSpacing: '0.18em', color, marginTop: 4, textAlign: 'center' }}>
        {name ? name.toUpperCase() : '—'}
      </div>
      <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#475569', marginTop: 3 }}>
        {window.TYPE_DEFS?.[type]?.label || type}
      </div>
    </div>
  );
}

// ── REGION FORGE ──────────────────────────────────────────────────
function RegionForge({ regions, onAdd, onUpdate, onDelete, selectedId }) {
  const editing = selectedId ? regions.find(r => r.id === selectedId) : null;

  const blank = { name: '', type: 'void-zone', x: 50, y: 50, rx: 8, ry: 6, hz: 432, stratum: 1, lore: '' };
  const [form, setForm] = React.useState(editing || blank);
  const [saved, setSaved] = React.useState(false);

  React.useEffect(() => {
    setForm(editing || blank);
    setSaved(false);
  }, [selectedId]);

  const set = (k, v) => setForm(f => ({ ...f, [k]: v }));

  const typeDef = window.TYPE_DEFS?.[form.type] || { color: '#00e5ff', glow: 'rgba(0,229,255,0.5)' };

  const handleSubmit = () => {
    const id = editing ? editing.id : window.genId('rgn');
    const region = {
      ...form,
      id,
      color: typeDef.color,
      glow: typeDef.glow,
      stratum: typeDef.stratum ?? form.stratum,
    };
    if (editing) {
      onUpdate(region);
    } else {
      onAdd(region);
    }
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
    if (!editing) setForm(blank);
  };

  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1fr 320px', gap: 28, alignItems: 'start' }}>
      {/* Form */}
      <div>
        <div style={{ fontFamily: 'Cinzel, serif', fontSize: 12, letterSpacing: '0.2em', color: '#fbbf24', marginBottom: 20 }}>
          {editing ? 'MODIFY REGION' : 'FORGE NEW REGION'}
        </div>

        <ForgeInput label="REGION NAME" value={form.name} onChange={v => set('name', v)} placeholder="Name this territory..."/>
        <ForgeSelect label="ENTITY TYPE" value={form.type} onChange={v => set('type', v)}
          options={Object.entries(window.TYPE_DEFS || {}).map(([k, v]) => ({ value: k, label: v.label }))}
        />
        <ForgeTextarea label="LORE · CARTOGRAPHIC NOTE" value={form.lore} onChange={v => set('lore', v)}
          placeholder="Describe this region's nature, history, and resonance..."
        />

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
          <ForgeSlider label="POSITION X" value={form.x} onChange={v => set('x', v)} min={2} max={97} unit="%"/>
          <ForgeSlider label="POSITION Y" value={form.y} onChange={v => set('y', v)} min={2} max={95} unit="%"/>
          <ForgeSlider label="EXTENT RX" value={form.rx} onChange={v => set('rx', v)} min={2} max={20} unit="qu"/>
          <ForgeSlider label="EXTENT RY" value={form.ry} onChange={v => set('ry', v)} min={2} max={16} unit="qu"/>
          <ForgeSlider label="RESONANCE HZ" value={form.hz} onChange={v => set('hz', v)} min={20} max={999} unit="hz"/>
          <ForgeSlider label="STRATUM" value={form.stratum} onChange={v => set('stratum', v)} min={0} max={9}/>
        </div>

        <div style={{ display: 'flex', gap: 10, marginTop: 8, alignItems: 'center' }}>
          <button onClick={handleSubmit} style={FORGE_BTN(typeDef.color, typeDef.glow)}
            onMouseEnter={e => e.currentTarget.style.background = `${typeDef.color}28`}
            onMouseLeave={e => e.currentTarget.style.background = `${typeDef.color}18`}
          >
            {editing ? '⚙ UPDATE REGION' : '⚜ FORGE REGION'}
          </button>
          {editing && (
            <button onClick={() => { onDelete(editing.id); }}
              style={{ ...FORGE_BTN('#f97316', 'rgba(249,115,22,0.3)'), fontSize: 10 }}
              onMouseEnter={e => e.currentTarget.style.background = 'rgba(249,115,22,0.25)'}
              onMouseLeave={e => e.currentTarget.style.background = 'rgba(249,115,22,0.1)'}
            >✕ DELETE</button>
          )}
          {saved && (
            <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 9, color: '#39ff14', letterSpacing: '0.15em' }}>
              ✓ BOUND TO ATLAS
            </span>
          )}
        </div>
      </div>

      {/* Preview + region list */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
        <RegionPreview type={form.type} name={form.name} hz={form.hz} rx={form.rx} ry={form.ry}/>

        <div>
          <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 9, letterSpacing: '0.18em', color: '#475569', marginBottom: 8 }}>
            EXISTING REGIONS · {regions.length}
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, maxHeight: 320, overflowY: 'auto' }}>
            {regions.map(r => (
              <div key={r.id}
                onClick={() => onUpdate && window.__setForgeSelectedRegion && window.__setForgeSelectedRegion(r.id)}
                style={{
                  display: 'flex', alignItems: 'center', gap: 8,
                  padding: '6px 10px', borderRadius: 3, cursor: 'pointer',
                  background: selectedId === r.id ? `${r.color}14` : 'rgba(3,5,10,0.5)',
                  border: `1px solid ${selectedId === r.id ? r.color + '55' : 'rgba(120,180,255,0.07)'}`,
                  transition: 'all 150ms',
                }}
                onMouseEnter={e => { if (selectedId !== r.id) e.currentTarget.style.borderColor = `${r.color}33`; }}
                onMouseLeave={e => { if (selectedId !== r.id) e.currentTarget.style.borderColor = 'rgba(120,180,255,0.07)'; }}
              >
                <div style={{ width: 8, height: 8, borderRadius: '50%', background: r.color, boxShadow: `0 0 6px ${r.color}`, flexShrink: 0 }}/>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontFamily: 'Cinzel, serif', fontSize: 9, color: r.color, letterSpacing: '0.12em', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{r.name}</div>
                  <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 7, color: '#334155' }}>{r.type} · {r.hz}HZ</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── ROUTE FORGE ───────────────────────────────────────────────────
function RouteForge({ regions, routes, onAdd, onDelete }) {
  const blank = { from: '', to: '', type: 'quantum', label: '', hz: 432 };
  const [form, setForm] = React.useState(blank);
  const [saved, setSaved] = React.useState(false);
  const set = (k, v) => setForm(f => ({ ...f, [k]: v }));

  const rtDef = window.ROUTE_TYPE_DEFS?.[form.type] || { color: '#00e5ff', label: 'Thread' };
  const fromRegion = regions.find(r => r.id === form.from);
  const toRegion   = regions.find(r => r.id === form.to);

  const handleSubmit = () => {
    if (!form.from || !form.to || form.from === form.to) return;
    onAdd({
      id: window.genId('rt'),
      ...form,
      color: rtDef.color,
      opacity: 0.4,
    });
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
    setForm(blank);
  };

  const regionOpts = [{ value: '', label: '— SELECT REGION —' }, ...regions.map(r => ({ value: r.id, label: r.name }))];

  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1fr 320px', gap: 28, alignItems: 'start' }}>
      <div>
        <div style={{ fontFamily: 'Cinzel, serif', fontSize: 12, letterSpacing: '0.2em', color: '#fbbf24', marginBottom: 20 }}>
          WEAVE NEW ROUTE
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
          <ForgeSelect label="ORIGIN REGION" value={form.from} onChange={v => set('from', v)} options={regionOpts}/>
          <ForgeSelect label="TERMINUS REGION" value={form.to} onChange={v => set('to', v)} options={regionOpts}/>
        </div>

        <ForgeSelect label="THREAD TYPE" value={form.type} onChange={v => set('type', v)}
          options={Object.entries(window.ROUTE_TYPE_DEFS || {}).map(([k, v]) => ({ value: k, label: v.label }))}
        />

        <ForgeInput label="ROUTE LABEL" value={form.label} onChange={v => set('label', v)} placeholder="Name this thread..."/>
        <ForgeSlider label="RESONANCE HZ" value={form.hz} onChange={v => set('hz', v)} min={20} max={999} unit="hz"/>

        <div style={{ display: 'flex', gap: 10, marginTop: 8, alignItems: 'center' }}>
          <button onClick={handleSubmit}
            style={FORGE_BTN(rtDef.color, `${rtDef.color}55`)}
            onMouseEnter={e => e.currentTarget.style.background = `${rtDef.color}28`}
            onMouseLeave={e => e.currentTarget.style.background = `${rtDef.color}18`}
          >⟿ BIND ROUTE</button>
          {saved && (
            <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 9, color: '#39ff14', letterSpacing: '0.15em' }}>
              ✓ THREAD WOVEN
            </span>
          )}
        </div>
      </div>

      {/* Route preview + list */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
        {/* Preview thread */}
        <div style={{
          padding: '20px 16px',
          background: 'rgba(3,5,10,0.6)',
          border: `1px solid ${rtDef.color}22`,
          borderRadius: 6,
        }}>
          <svg width="280" height="80" viewBox="0 0 280 80">
            <defs>
              <marker id="arrowFwd" markerWidth="6" markerHeight="6" refX="3" refY="3" orient="auto">
                <path d="M0,0 L6,3 L0,6 Z" fill={rtDef.color} opacity="0.8"/>
              </marker>
            </defs>
            {/* Origin */}
            <circle cx="40" cy="40" r="12"
              fill={fromRegion?.color || '#1e293b'}
              fillOpacity="0.2"
              stroke={fromRegion?.color || '#334155'} strokeWidth="1.2"/>
            <circle cx="40" cy="40" r="4" fill={fromRegion?.color || '#334155'}/>
            <text x="40" y="62" textAnchor="middle" fontFamily="JetBrains Mono, monospace" fontSize="7"
              fill={fromRegion?.color || '#475569'} letterSpacing="0.5">
              {fromRegion ? fromRegion.name.substring(0,8).toUpperCase() : 'ORIGIN'}
            </text>

            {/* Thread */}
            <path d="M55,40 Q140,15 225,40"
              fill="none" stroke={rtDef.color} strokeWidth="1.2"
              strokeDasharray="5 4" opacity="0.7"
              markerEnd="url(#arrowFwd)"
            />
            <text x="140" y="26" textAnchor="middle" fontFamily="JetBrains Mono, monospace" fontSize="7"
              fill={rtDef.color} opacity="0.65">{form.label || rtDef.label}</text>

            {/* Terminus */}
            <circle cx="240" cy="40" r="12"
              fill={toRegion?.color || '#1e293b'}
              fillOpacity="0.2"
              stroke={toRegion?.color || '#334155'} strokeWidth="1.2"/>
            <circle cx="240" cy="40" r="4" fill={toRegion?.color || '#334155'}/>
            <text x="240" y="62" textAnchor="middle" fontFamily="JetBrains Mono, monospace" fontSize="7"
              fill={toRegion?.color || '#475569'} letterSpacing="0.5">
              {toRegion ? toRegion.name.substring(0,8).toUpperCase() : 'TERMINUS'}
            </text>
          </svg>
          <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: rtDef.color, textAlign: 'center', marginTop: 4, letterSpacing: '0.15em' }}>
            {form.hz}HZ · {window.ROUTE_TYPE_DEFS?.[form.type]?.label?.toUpperCase() || 'THREAD'}
          </div>
        </div>

        {/* Route list */}
        <div>
          <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 9, letterSpacing: '0.18em', color: '#475569', marginBottom: 8 }}>
            ACTIVE ROUTES · {routes.length}
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, maxHeight: 280, overflowY: 'auto' }}>
            {routes.map(rt => {
              const fr = regions.find(r => r.id === rt.from);
              const to = regions.find(r => r.id === rt.to);
              return (
                <div key={rt.id} style={{
                  display: 'flex', alignItems: 'center', gap: 8,
                  padding: '6px 10px', borderRadius: 3,
                  background: 'rgba(3,5,10,0.5)',
                  border: `1px solid ${rt.color}18`,
                }}>
                  <div style={{ width: 6, height: 6, borderRadius: '50%', background: rt.color, flexShrink: 0 }}/>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ fontFamily: 'Cinzel, serif', fontSize: 9, color: rt.color, letterSpacing: '0.1em' }}>
                      {rt.label || rt.type.toUpperCase()}
                    </div>
                    <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 7, color: '#334155' }}>
                      {fr?.name || rt.from} → {to?.name || rt.to}
                    </div>
                  </div>
                  <button onClick={() => onDelete(rt.id)} style={{
                    background: 'transparent', border: '1px solid rgba(249,115,22,0.2)',
                    color: '#f97316', borderRadius: 2, cursor: 'pointer',
                    fontFamily: 'JetBrains Mono, monospace', fontSize: 8, padding: '2px 6px',
                    flexShrink: 0,
                  }}>✕</button>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── SURVEY FORGE ──────────────────────────────────────────────────
function SurveyForge({ survey, onAdd, onDelete }) {
  const SURVEY_COLORS = [
    { label: 'Quantum Cyan', value: '#00e5ff' },
    { label: 'Bio Green',    value: '#39ff14' },
    { label: 'Mythos Violet',value: '#c084fc' },
    { label: 'Forge Gold',   value: '#fbbf24' },
    { label: 'Ember Orange', value: '#f97316' },
  ];
  const PREFIXES = ['QSV', 'BSV', 'MSV', 'GSV', 'ESV', 'CSV', 'PSV'];

  const nextLabel = () => {
    const nums = survey.map(s => parseInt(s.label.split('·')[1] || '0')).filter(Boolean);
    const next = nums.length ? Math.max(...nums) + 1 : 1;
    return `QSV·${String(next).padStart(3, '0')}`;
  };

  const blank = { x: 50, y: 50, color: '#00e5ff', label: nextLabel(), note: '' };
  const [form, setForm] = React.useState(blank);
  const [saved, setSaved] = React.useState(false);
  const set = (k, v) => setForm(f => ({ ...f, [k]: v }));

  const handleSubmit = () => {
    onAdd({ id: window.genId('sv'), ...form });
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
    setForm({ ...blank, label: nextLabel() });
  };

  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1fr 320px', gap: 28, alignItems: 'start' }}>
      <div>
        <div style={{ fontFamily: 'Cinzel, serif', fontSize: 12, letterSpacing: '0.2em', color: '#fbbf24', marginBottom: 20 }}>
          PLACE SURVEY MARKER
        </div>

        <ForgeInput label="MARKER LABEL" value={form.label} onChange={v => set('label', v)} placeholder="QSV·001"/>
        <ForgeTextarea label="SURVEY NOTE" value={form.note} onChange={v => set('note', v)}
          placeholder="Record observations, anomalies, classifications..." rows={3}
        />

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
          <ForgeSlider label="POSITION X" value={form.x} onChange={v => set('x', v)} min={1} max={99} unit="%"/>
          <ForgeSlider label="POSITION Y" value={form.y} onChange={v => set('y', v)} min={1} max={99} unit="%"/>
        </div>

        {/* Color picker */}
        <div style={{ marginBottom: 14 }}>
          <span style={FORGE_LABEL_STYLE}>MARKER COLOR</span>
          <div style={{ display: 'flex', gap: 8 }}>
            {SURVEY_COLORS.map(c => (
              <div key={c.value}
                onClick={() => set('color', c.value)}
                title={c.label}
                style={{
                  width: 28, height: 28, borderRadius: 3, background: c.value,
                  border: `2px solid ${form.color === c.value ? 'white' : 'transparent'}`,
                  boxShadow: form.color === c.value ? `0 0 10px ${c.value}` : 'none',
                  cursor: 'pointer', transition: 'all 150ms',
                  opacity: form.color === c.value ? 1 : 0.5,
                }}
              />
            ))}
          </div>
        </div>

        <div style={{ display: 'flex', gap: 10, marginTop: 8, alignItems: 'center' }}>
          <button onClick={handleSubmit}
            style={FORGE_BTN(form.color, `${form.color}55`)}
            onMouseEnter={e => e.currentTarget.style.background = `${form.color}28`}
            onMouseLeave={e => e.currentTarget.style.background = `${form.color}18`}
          >⊕ PLANT SURVEY MARK</button>
          {saved && (
            <span style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 9, color: '#39ff14', letterSpacing: '0.15em' }}>
              ✓ MARK PLANTED
            </span>
          )}
        </div>
      </div>

      {/* Preview + list */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
        {/* Placement preview */}
        <div style={{
          padding: '16px', borderRadius: 6,
          background: 'rgba(3,5,10,0.6)',
          border: `1px solid ${form.color}22`,
        }}>
          <svg width="280" height="120" viewBox="0 0 280 120">
            <rect width="280" height="120" fill="#060c1a" rx="4"/>
            {/* Mini map bg */}
            {Array.from({length:8},(_,i)=>(
              <line key={`v${i}`} x1={i*40} y1={0} x2={i*40} y2={120} stroke="#00e5ff" strokeWidth="0.3" opacity="0.06"/>
            ))}
            {Array.from({length:4},(_,i)=>(
              <line key={`h${i}`} x1={0} y1={i*40} x2={280} y2={i*40} stroke="#00e5ff" strokeWidth="0.3" opacity="0.06"/>
            ))}
            {/* Point */}
            <line x1={form.x*2.8-5} y1={form.y*1.2} x2={form.x*2.8+5} y2={form.y*1.2} stroke={form.color} strokeWidth="1.2" opacity="0.9"/>
            <line x1={form.x*2.8} y1={form.y*1.2-5} x2={form.x*2.8} y2={form.y*1.2+5} stroke={form.color} strokeWidth="1.2" opacity="0.9"/>
            <circle cx={form.x*2.8} cy={form.y*1.2} r="3" fill={form.color} style={{ filter: `drop-shadow(0 0 4px ${form.color})` }}/>
            <text x={form.x*2.8+6} y={form.y*1.2-4}
              fontFamily="JetBrains Mono, monospace" fontSize="7" fill={form.color} opacity="0.8">{form.label}</text>
          </svg>
          <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#475569', marginTop: 4 }}>
            POSITION · {form.x}%, {form.y}%
          </div>
        </div>

        {/* Survey list */}
        <div>
          <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 9, letterSpacing: '0.18em', color: '#475569', marginBottom: 8 }}>
            ACTIVE MARKS · {survey.length}
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, maxHeight: 280, overflowY: 'auto' }}>
            {survey.map(sv => (
              <div key={sv.id} style={{
                display: 'flex', alignItems: 'center', gap: 8,
                padding: '6px 10px', borderRadius: 3,
                background: 'rgba(3,5,10,0.5)',
                border: `1px solid ${sv.color}18`,
              }}>
                <div style={{ width: 6, height: 6, borderRadius: '50%', background: sv.color, boxShadow: `0 0 6px ${sv.color}`, flexShrink: 0 }}/>
                <div style={{ flex: 1 }}>
                  <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 9, color: sv.color }}>{sv.label}</div>
                  <div style={{ fontFamily: 'Space Grotesk, sans-serif', fontSize: 9, color: '#334155', marginTop: 1 }}>{sv.note}</div>
                </div>
                <button onClick={() => onDelete(sv.id)} style={{
                  background: 'transparent', border: '1px solid rgba(249,115,22,0.2)',
                  color: '#f97316', borderRadius: 2, cursor: 'pointer',
                  fontFamily: 'JetBrains Mono, monospace', fontSize: 8, padding: '2px 6px', flexShrink: 0,
                }}>✕</button>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Forge Screen wrapper ───────────────────────────────────────────
const FORGE_TABS = [
  { id: 'region', label: 'REGION FORGE', glyph: '⬡', color: '#00e5ff', motto: '"From void, territory. From territory, mandate."' },
  { id: 'route',  label: 'ROUTE WEAVE',  glyph: '⟿', color: '#c084fc', motto: '"Every thread is a promise between two places."' },
  { id: 'survey', label: 'SURVEY MARK',  glyph: '⊕', color: '#39ff14', motto: '"What is measured can be bound. What is bound, known."' },
];

function ForgeScreen({ world, onWorldChange }) {
  const [tab, setTab] = React.useState('region');
  const [selectedRegion, setSelectedRegion] = React.useState(null);
  window.__setForgeSelectedRegion = setSelectedRegion;

  const activeTab = FORGE_TABS.find(t => t.id === tab);

  const addRegion    = (r) => onWorldChange({ ...world, regions: [...world.regions, r] });
  const updateRegion = (r) => onWorldChange({ ...world, regions: world.regions.map(x => x.id === r.id ? r : x) });
  const deleteRegion = (id) => { onWorldChange({ ...world, regions: world.regions.filter(r => r.id !== id) }); setSelectedRegion(null); };

  const addRoute     = (rt) => onWorldChange({ ...world, routes: [...world.routes, rt] });
  const deleteRoute  = (id) => onWorldChange({ ...world, routes: world.routes.filter(r => r.id !== id) });

  const addSurvey    = (sv) => onWorldChange({ ...world, survey: [...world.survey, sv] });
  const deleteSurvey = (id) => onWorldChange({ ...world, survey: world.survey.filter(s => s.id !== id) });

  return (
    <div style={{
      flex: 1, display: 'flex', flexDirection: 'column',
      background: 'rgba(3,5,10,0.6)',
      overflow: 'hidden',
    }}>
      {/* Forge header */}
      <div style={{
        padding: '18px 32px 0',
        borderBottom: '1px solid rgba(251,191,36,0.12)',
        flexShrink: 0,
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
          <div style={{ fontFamily: 'Cinzel Decorative, serif', fontSize: 18, letterSpacing: '0.12em', color: '#fbbf24', textShadow: '0 0 24px rgba(251,191,36,0.3)' }}>
            THE FORGE
          </div>
          <div style={{ height: 1, flex: 1, background: 'linear-gradient(90deg, rgba(251,191,36,0.3), transparent)' }}/>
          <div style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 8, color: '#475569' }}>
            {world.regions.length} REGIONS · {world.routes.length} ROUTES · {world.survey.length} MARKS
          </div>
        </div>

        {/* Forge tabs */}
        <div style={{ display: 'flex', gap: 2 }}>
          {FORGE_TABS.map(ft => {
            const active = tab === ft.id;
            return (
              <button key={ft.id} onClick={() => setTab(ft.id)} style={{
                display: 'flex', alignItems: 'center', gap: 7,
                padding: '10px 20px',
                background: active ? `${ft.color}12` : 'transparent',
                border: 'none',
                borderBottom: `2px solid ${active ? ft.color : 'transparent'}`,
                color: active ? ft.color : '#475569',
                fontFamily: 'Cinzel, serif', fontSize: 10,
                letterSpacing: '0.18em', cursor: 'pointer',
                transition: 'all 180ms',
              }}
                onMouseEnter={e => { if (!active) e.currentTarget.style.color = ft.color; }}
                onMouseLeave={e => { if (!active) e.currentTarget.style.color = '#475569'; }}
              >
                <span style={{ fontSize: 13 }}>{ft.glyph}</span>
                {ft.label}
              </button>
            );
          })}
        </div>
      </div>

      {/* Motto strip */}
      <div style={{
        padding: '10px 32px',
        borderBottom: '1px solid rgba(120,180,255,0.05)',
        flexShrink: 0,
      }}>
        <div style={{ fontFamily: 'Cormorant Garamond, serif', fontStyle: 'italic', fontSize: 13, color: '#475569' }}>
          {activeTab?.motto}
        </div>
      </div>

      {/* Content */}
      <div style={{ flex: 1, overflowY: 'auto', padding: '28px 32px' }}>
        {tab === 'region' && (
          <RegionForge
            regions={world.regions}
            onAdd={addRegion}
            onUpdate={updateRegion}
            onDelete={deleteRegion}
            selectedId={selectedRegion}
          />
        )}
        {tab === 'route' && (
          <RouteForge
            regions={world.regions}
            routes={world.routes}
            onAdd={addRoute}
            onDelete={deleteRoute}
          />
        )}
        {tab === 'survey' && (
          <SurveyForge
            survey={world.survey}
            onAdd={addSurvey}
            onDelete={deleteSurvey}
          />
        )}
      </div>
    </div>
  );
}

Object.assign(window, { ForgeScreen });
