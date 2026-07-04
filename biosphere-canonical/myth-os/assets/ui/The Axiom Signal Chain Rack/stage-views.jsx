/* global React */
// ═══════════════════════════════════════════════════════════════════
// STAGE VIEWS · the dive
// InstrumentRail · DiveNav · L2 EuroSynth grid · L3 Atom Library
// ═══════════════════════════════════════════════════════════════════
const { useState: useS, useRef: useR, useEffect: useE } = React;

function hexA(hex, a) {
  const m = (hex || '#00bfff').replace('#', '');
  const n = parseInt(m.length === 3 ? m.split('').map(c => c + c).join('') : m, 16);
  return 'rgba(' + ((n >> 16) & 255) + ',' + ((n >> 8) & 255) + ',' + (n & 255) + ',' + a + ')';
}
const clmp = (v) => v < 0 ? 0 : v > 1 ? 1 : v;
function fmtVal(c) {
  if (c.type === 'toggle') return c.value ? 'ON' : 'OFF';
  if (c.type === 'select') return (c.opts && c.opts[c.value]) || '—';
  if (c.type === 'jack') return c.dir === 'in' ? '◂ IN' : 'OUT ▸';
  const v = c.min + (c.max - c.min) * c.value;
  const r = Math.abs(v) >= 100 ? Math.round(v) : Math.round(v * 10) / 10;
  return r + (c.unit ? c.unit : '');
}

// ── atom primitives (honor arbitrary wire colors) ───────────────────
function AtomKnob({ c, size, onChange }) {
  const [drag, setDrag] = useS(false); const sY = useR(0), sV = useR(c.value);
  const down = (e) => { e.preventDefault(); setDrag(true); sY.current = e.clientY; sV.current = c.value; e.currentTarget.setPointerCapture(e.pointerId); };
  const move = (e) => { if (!drag) return; onChange(clmp(sV.current + (sY.current - e.clientY) * (e.shiftKey ? 0.0015 : 0.005))); };
  const up = (e) => { setDrag(false); try { e.currentTarget.releasePointerCapture(e.pointerId); } catch {} };
  const ang = -135 + c.value * 270;
  return (
    <div className={`knob ${size || 'sm'}`} style={{ '--d': c.color, '--d-g': hexA(c.color, .5) }}
      onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={up} onDoubleClick={() => onChange(0.5)}>
      <div className="cap" /><div className="ind" style={{ transform: `rotate(${ang}deg)` }} />
    </div>
  );
}
function AtomFader({ c, height, onChange }) {
  const ref = useR(null); const [drag, setDrag] = useS(false);
  const set = (cy) => { const r = ref.current.getBoundingClientRect(); onChange(clmp(1 - (cy - r.top) / r.height)); };
  const down = (e) => { e.preventDefault(); setDrag(true); set(e.clientY); e.currentTarget.setPointerCapture(e.pointerId); };
  const move = (e) => { if (drag) set(e.clientY); };
  const up = (e) => { setDrag(false); try { e.currentTarget.releasePointerCapture(e.pointerId); } catch {} };
  return (
    <div className="fader-track" ref={ref} style={{ height: height || 64, '--d': c.color, '--d-g': hexA(c.color, .5) }}
      onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={up}>
      <div className="lit" style={{ height: `${c.value * 100}%` }} /><div className="fader-thumb" style={{ bottom: `calc(${c.value * 100}% - 7px)` }} />
    </div>
  );
}
function AtomControl({ c, onChange, size, faderH }) {
  if (c.type === 'knob') return <AtomKnob c={c} size={size} onChange={onChange} />;
  if (c.type === 'fader') return <AtomFader c={c} height={faderH} onChange={onChange} />;
  if (c.type === 'toggle') return <div className={`atomtog ${c.value ? 'on' : ''}`} style={{ '--d': c.color }} onClick={() => onChange(!c.value)}>{c.value ? 'ON' : 'OFF'}</div>;
  if (c.type === 'pad') return <div className="atompad" style={{ '--pc': c.color }} onPointerDown={() => onChange(true)} />;
  if (c.type === 'select') return <div className="atomsel" style={{ '--d': c.color }} onClick={() => onChange(((c.value | 0) + 1) % (c.opts ? c.opts.length : 1))}>{(c.opts && c.opts[c.value]) || '—'}</div>;
  if (c.type === 'jack') return <div className={`atomjack ${c.dir}`} style={{ '--d': c.color }} title={c.label} />;
  return null;
}

// ── INSTRUMENT RAIL (persistent) ────────────────────────────────────
function InstrumentRail({ instruments, activeId, onSelect }) {
  return (
    <div className="irail">
      {instruments.map((ins, i) => {
        const col = `var(--${ins.ch})`;
        return (
          <div key={ins.id} className={`irow ${activeId === ins.id ? 'active' : ''} ${ins.status}`} style={{ '--ic': col }} onClick={() => onSelect(ins.id)}>
            <span className="inum">{String(i + 1).padStart(2, '0')}</span>
            <span className="icrest" style={{ background: `radial-gradient(circle at 32% 30%, #fff, ${col})`, boxShadow: `0 0 7px ${col}` }} />
            <span className="iname">{ins.name}</span>
            <span className={`istat ${ins.status}`}>{ins.status === 'live' ? '●' : '◐'}</span>
          </div>
        );
      })}
    </div>
  );
}

// ── DIVE NAV (persistent) ───────────────────────────────────────────
const DEPTHS = [{ k: 1, l: 'PERFORM', x: 'Mythos' }, { k: 2, l: 'PATCH', x: '16 EuroSynths' }, { k: 3, l: 'ATELIER', x: 'Atom Library' }];
function DiveNav({ depth, setDepth, instrument, container }) {
  return (
    <div className="divenav">
      <div className="breadcrumb">
        <span className="bc bc-root">GENESIS</span>
        <span className="bc-sep">▸</span>
        <span className="bc bc-inst" style={{ color: `var(--${instrument.ch})` }}>{instrument.name}</span>
        {depth >= 2 && <><span className="bc-sep">▸</span><span className="bc">{depth === 2 ? '16 CONTAINERS' : (container ? container.name : 'CONTAINER')}</span></>}
        {depth >= 3 && container && <><span className="bc-sep">▸</span><span className="bc" style={{ color: container.color }}>{container.archetype}</span></>}
      </div>
      <div className="depthtabs">
        {DEPTHS.map(d => (
          <button key={d.k} className={`depthtab ${depth === d.k ? 'on' : ''}`} onClick={() => setDepth(d.k)} disabled={d.k === 3 && !container}>
            <span className="dt-l">L{d.k} · {d.l}</span>
            <span className="dt-x">{d.x}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

// ── L2 · EuroSynth module ───────────────────────────────────────────
function EuroModule({ container, selected, onSelect, onDive, onCtl }) {
  return (
    <div className={`euromod ${selected ? 'sel' : ''}`} style={{ '--mc': container.color }}
      onClick={() => onSelect(container.id)} onDoubleClick={() => onDive(container.id)}>
      <div className="em-screws"><span /><span /></div>
      <div className="em-head">
        <span className="em-name">{container.name}</span>
        <span className="em-wire" style={{ color: container.color }}>{container.wire}</span>
      </div>
      <div className="em-arch">{container.archetype}</div>
      <div className="em-body">
        {container.controls.slice(0, 16).map((c, i) => (
          <div key={c.id} className="em-ctl" title={`${c.label} · ${fmtVal(c)}`} onClick={(e) => e.stopPropagation()}>
            <AtomControl c={c} size="xs" faderH={42} onChange={(v) => onCtl(container.id, c.id, v)} />
            {c.type !== 'pad' && <span className="em-lbl">{c.label}</span>}
          </div>
        ))}
      </div>
      <div className="em-foot">
        <span className="em-io">{container.controls.filter(c => c.type === 'jack').length} I/O</span>
        <button className="em-dive" onClick={(e) => { e.stopPropagation(); onDive(container.id); }}>DIVE ⤓</button>
      </div>
    </div>
  );
}
function L2Grid({ containers, selectedId, onSelect, onDive, onCtl }) {
  return (
    <div className="euro-grid">
      {containers.map(c => <EuroModule key={c.id} container={c} selected={selectedId === c.id} onSelect={onSelect} onDive={onDive} onCtl={onCtl} />)}
    </div>
  );
}

// ── L3 · Atom card (editable) ───────────────────────────────────────
const TYPE_LABEL = { knob: 'ROTARY', fader: 'LINEAR', pad: 'TRIGGER', toggle: 'GATE', select: 'STEPPED', jack: 'PORT' };
const SWATCHES = ['#00bfff', '#ff2db5', '#fbbf24', '#39ff14', '#b06bff', '#f97316', '#14b8a6', '#fb7185'];
function AtomCard({ c, theme, onEdit, expanded, onExpand }) {
  const acc = c.color;
  return (
    <div className="atomcard" style={{ '--ac': acc, '--theme': theme }}>
      <div className="ac-top">
        <span className="ac-type" style={{ color: acc }}>{TYPE_LABEL[c.type] || 'ATOM'}</span>
        <input className="ac-label" value={c.label} onChange={e => onEdit(c.id, { label: e.target.value.toUpperCase().slice(0, 8) })} spellCheck={false} />
        <span className="ac-wire" style={{ background: hexA(acc, .16), color: acc }}>{c.wire}</span>
      </div>
      <div className="ac-stage">
        <AtomControl c={c} size="sm" faderH={70} onChange={(v) => onEdit(c.id, { value: v })} />
        <div className="ac-readout" style={{ color: acc }}>{fmtVal(c)}</div>
      </div>
      <div className="ac-range">
        <span className="ac-rl">RANGE</span>
        <input className="ac-num" type="number" value={c.min} onChange={e => onEdit(c.id, { min: parseFloat(e.target.value) || 0 })} />
        <span className="ac-dash">–</span>
        <input className="ac-num" type="number" value={c.max} onChange={e => onEdit(c.id, { max: parseFloat(e.target.value) || 0 })} />
        <input className="ac-unit" value={c.unit} onChange={e => onEdit(c.id, { unit: e.target.value.slice(0, 3) })} placeholder="unit" spellCheck={false} />
      </div>
      <div className="ac-swatches">
        {SWATCHES.map(s => <button key={s} className={`ac-sw ${c.color === s ? 'on' : ''}`} style={{ background: s }} onClick={() => onEdit(c.id, { color: s })} />)}
      </div>
      <div className="ac-script" onClick={() => onExpand(expanded ? null : c.id)}>
        <span className="ac-sk">ƒ</span>
        {expanded ? (
          <input className="ac-scin" value={c.script} onChange={e => onEdit(c.id, { script: e.target.value })} onClick={e => e.stopPropagation()} spellCheck={false} autoFocus />
        ) : (
          <span className="ac-sc">{c.script}</span>
        )}
      </div>
    </div>
  );
}
function L3Library({ container, theme, onEdit }) {
  const [exp, setExp] = useS(null);
  return (
    <div className="atelier">
      <div className="atelier-controls">
        {container.controls.map(c => <AtomCard key={c.id} c={c} theme={theme} onEdit={onEdit} expanded={exp === c.id} onExpand={setExp} />)}
      </div>
    </div>
  );
}

Object.assign(window, { InstrumentRail, DiveNav, L2Grid, L3Library, EuroModule, AtomCard, AtomControl, fmtVal, hexA });
