/* global React */
// ═══════════════════════════════════════════════════════════════════
// DECK · the MOLECULES — reusable control clusters
// Knob · Fader · JogWheel · PadGrid · LayerStack · ChannelStrip ·
// Sequencer · Transport · Theater
// ═══════════════════════════════════════════════════════════════════
const { useState, useRef, useEffect, useMemo, useCallback } = React;

const CH = {
  cy: { c: 'var(--cy)', g: 'var(--cy-g)' }, mg: { c: 'var(--mg)', g: 'var(--mg-g)' },
  gd: { c: 'var(--gd)', g: 'var(--gd-g)' }, gn: { c: 'var(--gn)', g: 'var(--gn-g)' },
  vi: { c: 'var(--vi)', g: 'rgba(176,107,255,.5)' },
};

// ── KNOB ────────────────────────────────────────────────────────────
function Knob({ label, value, onChange, size = '', ch = 'cy', fmt, detents = 0 }) {
  const [drag, setDrag] = useState(false);
  const sY = useRef(0), sV = useRef(value);
  const down = (e) => { e.preventDefault(); setDrag(true); sY.current = e.clientY; sV.current = value; e.currentTarget.setPointerCapture(e.pointerId); };
  const move = (e) => {
    if (!drag) return;
    const dy = sY.current - e.clientY;
    let n = clamp01(sV.current + dy * (e.shiftKey ? 0.0015 : 0.005));
    if (detents > 1) n = Math.round(n * (detents - 1)) / (detents - 1);
    onChange(n);
  };
  const up = (e) => { setDrag(false); try { e.currentTarget.releasePointerCapture(e.pointerId); } catch {} };
  const angle = -135 + value * 270;
  const col = CH[ch] || CH.cy;
  const ticks = [];
  const tk = detents > 1 ? detents : 11;
  for (let i = 0; i < tk; i++) {
    const a = (-135 + (i / (tk - 1)) * 270 - 90) * Math.PI / 180;
    const lit = (i / (tk - 1)) <= value + 0.001;
    ticks.push(<line key={i} x1={50 + Math.cos(a) * 44} y1={50 + Math.sin(a) * 44} x2={50 + Math.cos(a) * 39} y2={50 + Math.sin(a) * 39}
      stroke={lit ? col.c : 'rgba(120,140,170,.25)'} strokeWidth={lit ? 2 : 1.2} strokeLinecap="round" />);
  }
  const disp = fmt ? fmt(value) : Math.round(value * 100).toString().padStart(2, '0');
  return (
    <div className="knob-wrap">
      {label && <div className="knob-l">{label}</div>}
      <div className={`knob ${size} ${drag ? 'dragging' : ''}`} style={{ '--d': col.c, '--d-g': col.g }}
        onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={up} onDoubleClick={() => onChange(0.5)}>
        <svg className="knob-arc" viewBox="0 0 100 100">{ticks}</svg>
        <div className="cap" />
        <div className="ind" style={{ transform: `rotate(${angle}deg)` }} />
      </div>
      {fmt !== null && <div className="knob-v" style={{ '--d': col.c, '--d-g': col.g }}>{disp}</div>}
    </div>
  );
}

// ── FADER ───────────────────────────────────────────────────────────
function Fader({ label, value, onChange, ch = 'cy', height = 130, fmt }) {
  const ref = useRef(null); const [drag, setDrag] = useState(false);
  const set = (cy) => { const r = ref.current.getBoundingClientRect(); onChange(clamp01(1 - (cy - r.top) / r.height)); };
  const down = (e) => { e.preventDefault(); setDrag(true); set(e.clientY); e.currentTarget.setPointerCapture(e.pointerId); };
  const move = (e) => { if (drag) set(e.clientY); };
  const up = (e) => { setDrag(false); try { e.currentTarget.releasePointerCapture(e.pointerId); } catch {} };
  const col = CH[ch] || CH.cy;
  return (
    <div className="fader-wrap" style={{ '--d': col.c, '--d-g': col.g }}>
      <div className="fader-track" ref={ref} style={{ height }} onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={up}>
        <div className="scale" /><div className="lit" style={{ height: `${value * 100}%` }} />
        <div className="fader-thumb" style={{ bottom: `calc(${value * 100}% - 7px)` }} />
      </div>
      {label && <div className="fader-l">{label}</div>}
      <div className="fader-v">{fmt ? fmt(value) : Math.round(value * 100)}</div>
    </div>
  );
}
const clamp01 = (v) => v < 0 ? 0 : v > 1 ? 1 : v;

// ── JOG WHEEL (time scrub) ──────────────────────────────────────────
function JogWheel({ engine, time, era, epoch }) {
  const ref = useRef(null); const [drag, setDrag] = useState(false);
  const last = useRef(0); const center = useRef({ x: 0, y: 0 });
  const ang = (x, y) => Math.atan2(y - center.current.y, x - center.current.x);
  const down = (e) => {
    e.preventDefault(); setDrag(true);
    const r = ref.current.getBoundingClientRect(); center.current = { x: r.left + r.width / 2, y: r.top + r.height / 2 };
    last.current = ang(e.clientX, e.clientY); e.currentTarget.setPointerCapture(e.pointerId);
  };
  const move = (e) => {
    if (!drag) return;
    const a = ang(e.clientX, e.clientY); let d = a - last.current;
    if (d > Math.PI) d -= 2 * Math.PI; if (d < -Math.PI) d += 2 * Math.PI;
    last.current = a; engine && engine.scrub(d / (Math.PI * 2) * 0.5);
  };
  const up = (e) => { setDrag(false); try { e.currentTarget.releasePointerCapture(e.pointerId); } catch {} };
  return (
    <div className="jog" ref={ref} onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={up}>
      <div className="ring" />
      <div className="ringmask" />
      <div className="marker" style={{ transform: `rotate(${time * 360}deg)` }} />
      <div className="platter">
        <div className="grip" style={{ transform: `rotate(${time * 1440}deg)` }} />
        <div className="epoch-lbl">EPOCH</div>
        <div className="epoch-num">{epoch}</div>
        <div className="epoch-era">{era}</div>
      </div>
    </div>
  );
}

// ── TRANSPORT ───────────────────────────────────────────────────────
function Transport({ playing, reverse, loop, onPlay, onRev, onLoop }) {
  return (
    <div className="transport">
      <button className={`tbtn ${playing ? 'on' : ''}`} onClick={onPlay}>{playing ? '❚❚ Halt' : '▶ Weave'}</button>
      <button className={`tbtn rev ${reverse ? 'on' : ''}`} onClick={onRev}>◀◀ Aeon</button>
      <button className={`tbtn loop ${loop ? 'on' : ''}`} onClick={onLoop}>↻ Loop</button>
    </div>
  );
}

// ── PAD GRID ────────────────────────────────────────────────────────
const PAD_RGB = { cy: '0,191,255', mg: '255,45,181', gd: '251,191,36', gn: '57,255,20', vi: '176,107,255', or: '249,115,22', rd: '239,68,68', wt: '210,225,245' };
function PadGrid({ pads, onFire }) {
  const [flash, setFlash] = useState({});
  const fire = (p, i) => {
    setFlash(f => ({ ...f, [i]: true }));
    setTimeout(() => setFlash(f => ({ ...f, [i]: false })), 90);
    onFire(p);
  };
  return (
    <div className="pad-grid">
      {pads.map((p, i) => (
        <div key={i} className={`pad ${flash[i] ? 'flash' : ''}`} style={{ '--pc': PAD_RGB[p.rgb] || PAD_RGB.cy }} onPointerDown={() => fire(p, i)}>
          <span className="pn">{(i + 1).toString().padStart(2, '0')}</span>
          <span className="pl">{p.label}</span>
          <span className="ps">{p.sub}</span>
        </div>
      ))}
    </div>
  );
}

// ── LAYER STACK ─────────────────────────────────────────────────────
const BLENDS = ['normal', 'screen', 'add', 'multiply'];
function LayerStack({ layers, activeId, onSelect, onToggle, onSolo, onMute, onAdd, addable }) {
  // display top-of-stack first (reverse of render order)
  const ordered = [...layers].slice().reverse();
  return (
    <div>
      <div className="layer-list">
        {ordered.map((ly) => {
          const col = CH[ly.ch] || CH.cy;
          return (
            <div key={ly.id} className={`layer-row ${activeId === ly.id ? 'active' : ''}`} style={{ '--lc': col.c }} onClick={() => onSelect(ly.id)}>
              <div className={`layer-eye ${ly.visible ? '' : 'off'}`} onClick={(e) => { e.stopPropagation(); onToggle(ly.id); }} title="visibility">
                <div className="dot" style={{ '--lc': col.c }} />
              </div>
              <div className="layer-mid">
                <div className="layer-name">{ly.name}</div>
                <div className="layer-meta">
                  <span className="layer-blend" style={{ '--lc': col.c }}>{ly.blend}</span>
                  <span className="layer-op">{Math.round(ly.opacity * 100)}%</span>
                </div>
              </div>
              <div className="layer-solo">
                <button className={`minibtn s ${ly.solo ? 'on' : ''}`} onClick={(e) => { e.stopPropagation(); onSolo(ly.id); }} title="solo">S</button>
                <button className={`minibtn m ${!ly.visible ? 'on' : ''}`} onClick={(e) => { e.stopPropagation(); onToggle(ly.id); }} title="mute">M</button>
              </div>
            </div>
          );
        })}
      </div>
      {addable && addable.length > 0 && (
        <div className="layer-add">
          {addable.map(a => <button key={a.kind} className="addchip" onClick={() => onAdd(a)}>+ {a.name}</button>)}
        </div>
      )}
    </div>
  );
}

// ── CHANNEL STRIP (mixer for the active layer) ──────────────────────
function ChannelStrip({ layer, atoms, onParam, onOpacity, onIllum, onBlend, sends, onSend }) {
  if (!layer) return null;
  const col = CH[layer.ch] || CH.cy;
  const p = layer.params || {};
  return (
    <div className="cstrip" style={{ '--lc': col.c }}>
      <div className="cstrip-top">
        <div className="cstrip-name">{layer.name}</div>
        <div className="atom-tag" style={{ color: col.c }}>ATOM · LIVE</div>
      </div>

      {/* ATOM parameter knobs (hi/mid/low remapped per ATOM) */}
      <div className="atom-row">
        {atoms.map((a) => (
          <div key={a.k} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 3 }}>
            <Knob label={a.label} ch={a.ch || layer.ch} value={p[a.k] != null ? p[a.k] : 0.5} onChange={(v) => onParam(a.k, v)} size="sm" fmt={null} />
            <span className="atom-tag">{a.tag}</span>
          </div>
        ))}
      </div>

      {/* blend mode */}
      <div>
        <div className="panel-x" style={{ marginBottom: 5 }}>BLEND MODE</div>
        <div className="blend-sel">
          {BLENDS.map(b => (
            <div key={b} className={`blend-opt ${layer.blend === b ? 'on' : ''}`} style={{ '--lc': col.c }} onClick={() => onBlend(b)}>{b === 'normal' ? 'norm' : b}</div>
          ))}
        </div>
      </div>

      {/* opacity + illumination faders */}
      <div className="fader-bank">
        <Fader label="OPACITY" ch={layer.ch} value={layer.opacity} onChange={onOpacity} height={108} fmt={(v) => Math.round(v * 100)} />
        <Fader label="ILLUM" ch="gd" value={(layer.illum != null ? layer.illum : 1) / 1.5} onChange={(v) => onIllum(v * 1.5)} height={108} fmt={(v) => Math.round(v * 150)} />
      </div>

      {/* send to layer */}
      <div>
        <div className="panel-x" style={{ marginBottom: 5 }}>SEND PADS → LAYER</div>
        <div className="send-sel">
          {sends.map(s => (
            <div key={s.id} className={`send-chip ${s.id === layer.sendTarget ? 'on' : ''}`} style={{ '--lc': col.c }} onClick={() => onSend(s.id)}>{s.short}</div>
          ))}
        </div>
      </div>
    </div>
  );
}

// ── SEQUENCER ───────────────────────────────────────────────────────
function Sequencer({ steps, playhead, playing, onToggle }) {
  return (
    <div className="seq">
      {steps.map((on, i) => (
        <div key={i} className={`seqstep ${on ? 'on' : ''} ${playing && playhead === i ? 'play' : ''}`} onClick={() => onToggle(i)}>
          {i % 4 === 0 ? (i + 1) : ''}
        </div>
      ))}
    </div>
  );
}

// ── THEATER (mounts the WorldEngine) ────────────────────────────────
function Theater({ layers, era, epoch, hz, registerEngine, onTime }) {
  const cvRef = useRef(null); const wrapRef = useRef(null); const engRef = useRef(null);
  useEffect(() => {
    const eng = new window.WorldEngine(cvRef.current);
    engRef.current = eng;
    eng.onTime = onTime;
    const fit = () => {
      const r = wrapRef.current.getBoundingClientRect();
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      eng.resize(Math.round(r.width), Math.round(r.height));
      cvRef.current.style.width = r.width + 'px'; cvRef.current.style.height = r.height + 'px';
    };
    fit(); window.addEventListener('resize', fit);
    eng.setLayers(layers); eng.start();
    registerEngine(eng);
    return () => { eng.stop(); window.removeEventListener('resize', fit); };
  }, []);
  useEffect(() => { if (engRef.current) engRef.current.setLayers(layers); }, [layers]);
  return (
    <div className="theater" ref={wrapRef}>
      <canvas ref={cvRef} />
      <div className="scan" /><div className="glare" /><div className="vig" />
      <div className="t-hud tl">◈ AXIOM THEATER<br />OUT → BIOSPARK</div>
      <div className="t-hud tr">{hz} HZ<br />SSoT · LIVE</div>
      <div className="t-hud bl">/realities/this/now</div>
      <div className="t-hud br"><span className="t-era">{era}</span><br />EPOCH {epoch}</div>
    </div>
  );
}

Object.assign(window, { Knob, Fader, JogWheel, Transport, PadGrid, LayerStack, ChannelStrip, Sequencer, Theater });
