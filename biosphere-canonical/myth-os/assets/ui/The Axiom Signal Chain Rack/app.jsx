/* global React, ReactDOM */
// ═══════════════════════════════════════════════════════════════════
// THE AXIOM CONTROLLER · app  (v3 — persistent chrome + dive)
// Genesis frame never changes. Select one of 16 Mythos instruments,
// then dive: L1 Perform → L2 Patch (16 EuroSynths) → L3 Atelier (atoms).
// ═══════════════════════════════════════════════════════════════════
const { useState, useEffect, useMemo, useRef, useCallback } = React;
const { Knob, Fader, JogWheel, Transport, PadGrid, LayerStack, ChannelStrip, Sequencer, Theater,
  InstrumentRail, DiveNav, L2Grid, L3Library, AtomControl, fmtVal, hexA,
  TweaksPanel, TweakSection, TweakSelect, TweakToggle, TweakButton, useTweaks } = window;

const INSTRUMENTS = window.AXIOM_INSTRUMENTS;

// ATOM maps for theater-layer channel strips
const ATOM_MAP = {
  genesis: [{ k: 'land', label: 'LAND', tag: 'SEA·LVL', ch: 'cy' }, { k: 'heat', label: 'HEAT', tag: 'MAGMA', ch: 'gd' }, { k: 'life', label: 'LIFE', tag: 'FAUNA', ch: 'gn' }],
  vorthex: [{ k: 'coh', label: 'COHERE', tag: 'COH', ch: 'mg' }, { k: 'ali', label: 'ALIGN', tag: 'ALI', ch: 'cy' }, { k: 'sep', label: 'SPREAD', tag: 'SEP', ch: 'gd' }],
  flora: [{ k: 'hi', label: 'GROWTH', tag: 'RATE', ch: 'gn' }, { k: 'mid', label: 'ANGLE', tag: 'DEG', ch: 'gd' }, { k: 'low', label: 'DEPTH', tag: 'GEN', ch: 'cy' }],
  aether: [{ k: 'hi', label: 'SPEED', tag: 'VEL', ch: 'cy' }, { k: 'mid', label: 'SCALE', tag: 'NOISE', ch: 'vi' }, { k: 'low', label: 'GLOW', tag: 'LUM', ch: 'gn' }],
};
const PRIMARY_EVENT = { genesis: 'ERUPT', vorthex: 'SWARM', flora: 'BLOOM', aether: 'FLASH' };
const KIND_META = {
  aether: { name: 'AETHER FIELD', ch: 'cy', blend: 'screen', opacity: 0.45 },
  genesis: { name: 'GENESIS · ATLAS', ch: 'gd', blend: 'normal', opacity: 1 },
  vorthex: { name: 'VORTHEX SWARM', ch: 'mg', blend: 'screen', opacity: 0.9 },
  flora: { name: 'EMERGENCE FLORA', ch: 'gn', blend: 'normal', opacity: 0.9 },
};
const DEFAULT_PARAMS = {
  genesis: { land: 0.5, heat: 0.4, life: 0.6 }, vorthex: { coh: 0.5, ali: 0.5, sep: 0.5 },
  flora: { hi: 0.5, mid: 0.4, low: 0.5 }, aether: { hi: 0.5, mid: 0.4, low: 0.5 },
};
let _lid = 0;
function makeLayer(kind) {
  const m = KIND_META[kind];
  return { id: kind + '_' + (_lid++), kind, name: m.name, ch: m.ch, visible: true, solo: false, opacity: m.opacity, illum: 1, blend: m.blend, params: { ...DEFAULT_PARAMS[kind] }, sendTarget: null };
}
const ERAS = [[0.10, 'HADEAN'], [0.22, 'ARCHEAN'], [0.40, 'OCEANIC'], [0.58, 'VERDANT'], [0.74, 'FAUNAL'], [0.88, 'SENTIENT'], [1.01, 'NOÖSPHERE']];
const eraOf = (t) => { for (const [b, n] of ERAS) if (t < b) return n; return 'NOÖSPHERE'; };

// ── inline: macro strip for non-theater instruments ─────────────────
function MacroStrip({ inst, container, onCtl }) {
  if (!container) return <div className="mi-empty">Generating instrument…</div>;
  const macros = container.controls.filter(c => c.type === 'knob' || c.type === 'fader').slice(0, 6);
  return (
    <div className="cstrip" style={{ '--lc': `var(--${inst.ch})` }}>
      <div className="cstrip-top"><div className="cstrip-name">{inst.name}</div><div className="atom-tag" style={{ color: `var(--${inst.ch})` }}>HERO · MACRO</div></div>
      <div className="atom-row" style={{ flexWrap: 'wrap', gap: 14 }}>
        {macros.map(c => (
          <div key={c.id} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}>
            <AtomControl c={c} size="" faderH={84} onChange={(v) => onCtl(container.id, c.id, v)} />
            <span className="atom-tag">{c.label}</span>
            <span className="atom-tag" style={{ color: c.color }}>{fmtVal(c)}</span>
          </div>
        ))}
      </div>
      <div style={{ fontFamily: 'var(--font-script)', fontStyle: 'italic', fontSize: 13, color: 'var(--fg-3)', textAlign: 'center', marginTop: 6 }}>{inst.motto}</div>
      <div className="lcd" style={{ justifyContent: 'space-between' }}><span className="k">DIVE</span> L2 · PATCH → 16 CONTAINERS</div>
    </div>
  );
}

// ── inline: module inspector (L2 right) ─────────────────────────────
function ModuleInspector({ container, onCtl, onDive }) {
  if (!container) return <div className="mi-empty">Select a EuroSynth module to inspect — double-click to dive into its atoms.</div>;
  return (
    <div className="minspect">
      <div className="spread"><span className="mi-title" style={{ color: container.color }}>{container.name}</span><span className="panel-x">{container.archetype}</span></div>
      <div className="mi-grid">
        {container.controls.map(c => (
          <div key={c.id} className="mi-ctl">
            <AtomControl c={c} size={c.type === 'knob' ? '' : 'sm'} faderH={84} onChange={(v) => onCtl(container.id, c.id, v)} />
            <span className="mi-clbl">{c.label}</span>
            <span className="mi-cval" style={{ color: c.color }}>{fmtVal(c)}</span>
          </div>
        ))}
      </div>
      <button className="addchip" style={{ alignSelf: 'flex-start' }} onClick={() => onDive(container.id)}>DIVE TO ATELIER ⤓</button>
    </div>
  );
}

// ── inline: boot splash (Astral Gateway power-on) ───────────────────
const BOOT_LINES = ['SSoT LINK ············ OK', 'WORLD ENGINE ········· OK', 'NEXUS ROUTING ········ OK', '16 MYTHOS INSTRUMENTS · LIVE'];
function BootSplash({ onDone }) {
  const [leaving, setLeaving] = useState(false);
  const leave = useCallback(() => { setLeaving(l => { if (!l) setTimeout(onDone, 620); return true; }); }, [onDone]);
  useEffect(() => { const t = setTimeout(leave, 3100); return () => clearTimeout(t); }, [leave]);
  return (
    <div className={'boot' + (leaving ? ' leave' : '')} onClick={leave}>
      <div className="boot-vortex"></div>
      <div className="boot-core">
        <svg className="boot-mark" viewBox="0 0 100 100"><defs><linearGradient id="bgBoot" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stopColor="#fbbf24" /><stop offset="50%" stopColor="#ff2db5" /><stop offset="100%" stopColor="#00bfff" /></linearGradient></defs><circle cx="50" cy="50" r="33" fill="none" stroke="url(#bgBoot)" strokeWidth="3" /><circle cx="50" cy="50" r="6" fill="url(#bgBoot)" /><path d="M50 17v12M50 71v12M17 50h12M71 50h12" stroke="url(#bgBoot)" strokeWidth="3" strokeLinecap="round" /></svg>
        <div className="boot-name">THE AXIOM CONTROLLER</div>
        <div className="boot-sub">ASTRAL GATEWAY · POWER-ON SELF TEST</div>
        <div className="boot-lines">{BOOT_LINES.map((l, i) => <div key={i} className="boot-line" style={{ animationDelay: (0.55 + i * 0.42) + 's' }}>{l}</div>)}</div>
        <div className="boot-skip">TAP ANYWHERE TO ENTER</div>
      </div>
    </div>
  );
}

function App() {
  const [tw, setTweak] = useTweaks(window.__CTRL_TWEAKS);
  useEffect(() => { document.body.className = 'finish-' + tw.finish + (tw.scanlines ? '' : ' noscan') + (tw.crt ? '' : ' nocrt'); }, [tw]);
  const [booted, setBooted] = useState(() => tw.boot === false || (window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches));

  // render layers (drive the Theater monitor — persistent)
  const [layers, setLayers] = useState(() => { const b = ['aether', 'genesis', 'vorthex'].map(makeLayer); b[0].opacity = 0.45; return b; });
  const patchLayer = (id, p) => setLayers(ls => ls.map(l => l.id === id ? { ...l, ...p } : l));
  const patchParam = (id, k, v) => setLayers(ls => ls.map(l => l.id === id ? { ...l, params: { ...l.params, [k]: v } } : l));
  const toggleVis = (id) => setLayers(ls => ls.map(l => l.id === id ? { ...l, visible: !l.visible } : l));
  const toggleSolo = (id) => setLayers(ls => ls.map(l => l.id === id ? { ...l, solo: !l.solo } : { ...l, solo: false }));
  const layerPool = ['aether', 'genesis', 'vorthex', 'flora'];
  const addable = layerPool.filter(k => !layers.some(l => l.kind === k)).map(k => ({ kind: k, name: KIND_META[k].name.split(' ')[0] }));
  const addLayer = (a) => { const nl = makeLayer(a.kind); setLayers(ls => [...ls, nl]); setActiveInstId(a.kind); };

  // navigation
  const [activeInstId, setActiveInstId] = useState('genesis');
  const [depth, setDepth] = useState(1);
  const [selectedCid, setSelectedCid] = useState(null);
  const [cache, setCache] = useState(() => ({ genesis: window.axiomContainers('genesis') }));
  const [theme, setTheme] = useState('transparent');
  const activeInst = INSTRUMENTS.find(i => i.id === activeInstId) || INSTRUMENTS[0];
  useEffect(() => { setCache(c => c[activeInstId] ? c : { ...c, [activeInstId]: window.axiomContainers(activeInstId) }); }, [activeInstId]);
  const containers = cache[activeInstId] || [];
  const selectedContainer = containers.find(c => c.id === selectedCid) || null;

  const selectInstrument = (id) => { setActiveInstId(id); setSelectedCid(null); setDepth(d => d === 3 ? 2 : d); };
  const diveContainer = (cid) => { setSelectedCid(cid); setDepth(3); };

  const onCtlValue = useCallback((cid, ctlId, v) => {
    setCache(c => ({ ...c, [activeInstId]: (c[activeInstId] || []).map(ct => ct.id === cid ? { ...ct, controls: ct.controls.map(x => x.id === ctlId ? { ...x, value: v } : x) } : ct) }));
  }, [activeInstId]);
  const onCtlEdit = useCallback((ctlId, patch) => {
    setCache(c => ({ ...c, [activeInstId]: (c[activeInstId] || []).map(ct => ct.id === selectedCid ? { ...ct, controls: ct.controls.map(x => x.id === ctlId ? { ...x, ...patch } : x) } : ct) }));
  }, [activeInstId, selectedCid]);

  // theater layer that matches the active instrument (for channel strip)
  const theaterLayer = activeInst.theaterKind ? layers.find(l => l.kind === activeInst.theaterKind) : null;

  // engine + transport
  const engRef = useRef(null);
  const [time, setTime] = useState(0.30);
  const registerEngine = (e) => { engRef.current = e; e.setTime(0.30); window.__axiomEngine = e; };
  const onTime = useCallback((t) => setTime(t), []);
  const [playing, setPlaying] = useState(false);
  const [reverse, setReverse] = useState(false);
  const [loop, setLoop] = useState(false);
  const [tempo, setTempo] = useState(0.45);
  const baseRate = 0.05;
  useEffect(() => { if (engRef.current) { engRef.current.setPlaying(playing); engRef.current.rate = baseRate * (reverse ? -1 : 1); } }, [playing, reverse]);
  useEffect(() => { const e = engRef.current; if (!e) return; if (loop) { const t = e.time; e.setLoop(Math.max(0, t - 0.06), Math.min(1, t + 0.06)); } else e.setLoop(0, 1); }, [loop, time]);
  const fire = useCallback((evt) => { engRef.current && engRef.current.fireEvent(evt); }, []);

  const pads = [
    { label: 'ERUPT', sub: 'MAGMA', rgb: 'or', evt: 'ERUPT' }, { label: 'SHIP', sub: 'CRASH', rgb: 'gn', evt: 'SHIP' },
    { label: 'QUAKE', sub: 'RUPTURE', rgb: 'rd', evt: 'QUAKE' }, { label: 'FLASH', sub: 'NOVA', rgb: 'wt', evt: 'FLASH' },
    { label: 'SWARM', sub: 'RING', rgb: 'mg', evt: 'SWARM' }, { label: 'WEDGE', sub: 'TACTICAL', rgb: 'cy', evt: 'FORMATION' },
    { label: 'BLOOM', sub: 'REGROW', rgb: 'gn', evt: 'BLOOM' }, { label: 'FREEZE', sub: 'STASIS', rgb: 'vi', evt: 'FREEZE' },
  ];
  const firePad = (p) => { if (p.evt === 'FREEZE') setPlaying(false); fire(p.evt); };

  const [steps, setSteps] = useState(() => Array.from({ length: 16 }, (_, i) => [0, 4, 7, 10, 12].includes(i)));
  const [playhead, setPlayhead] = useState(0);
  const toggleStep = (i) => setSteps(s => s.map((v, idx) => idx === i ? !v : v));
  const stepsRef = useRef(steps); stepsRef.current = steps;
  const instRef = useRef(activeInst); instRef.current = activeInst;
  useEffect(() => {
    if (!playing) return;
    const bpm = 70 + tempo * 230; const ms = (60 / bpm / 4) * 1000;
    const id = setInterval(() => setPlayhead(p => { const np = (p + 1) % 16; if (stepsRef.current[np]) fire(PRIMARY_EVENT[instRef.current.theaterKind] || 'FLASH'); return np; }), ms);
    return () => clearInterval(id);
  }, [playing, tempo, fire]);

  // scaling
  const [scale, setScale] = useState(1);
  useEffect(() => { const fit = () => setScale(Math.min(window.innerWidth / 1560, window.innerHeight / 940)); fit(); window.addEventListener('resize', fit); return () => window.removeEventListener('resize', fit); }, []);

  const era = eraOf(time), gya = ((1 - time) * 4.5).toFixed(2), epoch = Math.round(time * 999).toString().padStart(3, '0'), hz = (300 + time * 132).toFixed(1);
  const sends = layers.map(l => ({ id: l.id, short: l.name.split(' ')[0].slice(0, 4) }));

  return (
    <div className="stage">
      <div className="deck-scale" style={{ transform: `scale(${scale})` }}>
        <div className="deck">
          <div className="mount tl" /><div className="mount tr" /><div className="mount bl" /><div className="mount br" />

          {/* TOP BAR — persistent */}
          <div className="topbar">
            <div className="tb-brand">
              <svg className="tb-mark" viewBox="0 0 100 100"><defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stopColor="#fbbf24" /><stop offset="50%" stopColor="#ff2db5" /><stop offset="100%" stopColor="#00bfff" /></linearGradient></defs><circle cx="50" cy="50" r="33" fill="none" stroke="url(#bg)" strokeWidth="3" /><circle cx="50" cy="50" r="6" fill="url(#bg)" /><path d="M50 17v12M50 71v12M17 50h12M71 50h12" stroke="url(#bg)" strokeWidth="3" strokeLinecap="round" /></svg>
              <div><div className="tb-name">THE AXIOM CONTROLLER</div><div className="tb-sub">GENESIS CONTAINER · SSoT LIVE · v3.0</div></div>
            </div>
            <div className="tb-readouts">
              <div className="lcd"><span className="dotled" /> LIVE</div>
              <div className="lcd cy"><span className="k">EPOCH</span> {epoch}</div>
              <div className="lcd gd"><span className="k">ERA</span> {era}</div>
              <div className="lcd mg"><span className="k">T</span> {gya} GYA</div>
              <div className="lcd"><span className="k">INST</span> {INSTRUMENTS.findIndex(i => i.id === activeInstId) + 1}/16</div>
              <div className="lcd gd"><span className="k">φ</span> {hz} HZ</div>
            </div>
          </div>

          {/* MAIN */}
          <div className="main">
            {/* LEFT — instrument rail + jog (persistent) */}
            <div className="col">
              <div className="panel" style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
                <div className="pscrew a" /><div className="pscrew b" />
                <div className="panel-h"><span className="panel-t">Instruments</span><span className="panel-x">16 · Mythos</span></div>
                <InstrumentRail instruments={INSTRUMENTS} activeId={activeInstId} onSelect={selectInstrument} />
              </div>
              <div className="panel jog-panel" style={{ flex: '0 0 auto' }}>
                <div className="pscrew a" /><div className="pscrew b" />
                <div className="panel-h" style={{ width: '100%' }}><span className="panel-t">Epoch Scrubber</span><span className="panel-x">Time · Aeon</span></div>
                <JogWheel engine={engRef.current} time={time} era={era} epoch={epoch} />
                <Transport playing={playing} reverse={reverse} loop={loop} onPlay={() => setPlaying(p => !p)} onRev={() => setReverse(r => !r)} onLoop={() => setLoop(l => !l)} />
                <div style={{ display: 'flex', gap: 16, alignItems: 'flex-start', justifyContent: 'center', paddingTop: 2 }}>
                  <Knob label="TEMPO" ch="gn" value={tempo} onChange={setTempo} size="sm" fmt={(v) => (70 + v * 230 | 0)} />
                  <Knob label="SCRUB" ch="cy" value={time} onChange={(v) => engRef.current && engRef.current.setTime(v)} size="sm" fmt={(v) => Math.round(v * 99)} />
                </div>
              </div>
            </div>

            {/* CENTER — theater (persistent) + dive nav + stage */}
            <div className="col">
              <div className="theater-wrap" style={{ flex: '0 0 312px' }}>
                <Theater layers={layers} era={era} epoch={epoch} hz={hz} registerEngine={registerEngine} onTime={onTime} />
              </div>
              <DiveNav depth={depth} setDepth={setDepth} instrument={activeInst} container={selectedContainer} />
              <div className="panel" style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
                <div className="panel-h">
                  <span className="panel-t">{depth === 1 ? 'Performance' : depth === 2 ? 'Patch · 16 Containers' : 'Atelier · Atom Library'}</span>
                  <span className="panel-x">{depth === 1 ? activeInst.name : depth === 2 ? 'dbl-click to dive' : (selectedContainer ? selectedContainer.archetype : '')}</span>
                </div>
                <div className="stage-body">
                  {depth === 1 && (
                    <div className="perform-fill">
                      <div className="panel sunk" style={{ boxShadow: 'var(--inset-deep)', flex: '0 0 auto' }}>
                        <div className="spread" style={{ marginBottom: 8 }}><span className="panel-t">Event Lane</span><span className="panel-x">fires {activeInst.theaterKind ? PRIMARY_EVENT[activeInst.theaterKind] : 'FLASH'}</span></div>
                        <Sequencer steps={steps} playhead={playhead} playing={playing} onToggle={toggleStep} />
                      </div>
                      <div className="pad-sect">
                        <div className="panel-h"><span className="panel-t">Performance Pads</span><span className="panel-x">spawn into world</span></div>
                        <PadGrid pads={pads} onFire={firePad} />
                      </div>
                    </div>
                  )}
                  {depth === 2 && <L2Grid containers={containers} selectedId={selectedCid} onSelect={setSelectedCid} onDive={diveContainer} onCtl={onCtlValue} />}
                  {depth === 3 && selectedContainer && <L3Library container={selectedContainer} theme={theme} onEdit={onCtlEdit} />}
                </div>
              </div>
            </div>

            {/* RIGHT — layer stack (persistent) + depth inspector */}
            <div className="col">
              <div className="panel" style={{ flex: '0 0 auto' }}>
                <div className="pscrew a" /><div className="pscrew b" />
                <div className="panel-h"><span className="panel-t">Layer Stack</span><span className="panel-x">composite → theater</span></div>
                <LayerStack layers={layers} activeId={theaterLayer ? theaterLayer.id : null} onSelect={(id) => { const l = layers.find(x => x.id === id); if (l) setActiveInstId(l.kind); }} onToggle={toggleVis} onSolo={toggleSolo} onMute={toggleVis} onAdd={addLayer} addable={addable} />
              </div>
              <div className="panel" style={{ flex: 1, minHeight: 0, overflowY: 'auto' }}>
                <div className="pscrew a" /><div className="pscrew b" />
                <div className="panel-h">
                  <span className="panel-t">{depth === 1 ? 'Channel Strip' : depth === 2 ? 'Module Inspector' : 'Kit & Capsule'}</span>
                  <span className="panel-x">{depth === 1 ? 'active' : depth === 2 ? 'selected' : 'author'}</span>
                </div>
                {depth === 1 && (theaterLayer
                  ? <ChannelStrip layer={theaterLayer} atoms={ATOM_MAP[theaterLayer.kind]} onParam={(k, v) => patchParam(theaterLayer.id, k, v)} onOpacity={(v) => patchLayer(theaterLayer.id, { opacity: v })} onIllum={(v) => patchLayer(theaterLayer.id, { illum: v })} onBlend={(b) => patchLayer(theaterLayer.id, { blend: b })} sends={sends} onSend={(id) => patchLayer(theaterLayer.id, { sendTarget: id })} />
                  : <MacroStrip inst={activeInst} container={containers[0]} onCtl={onCtlValue} />)}
                {depth === 2 && <ModuleInspector container={selectedContainer} onCtl={onCtlValue} onDive={diveContainer} />}
                {depth === 3 && selectedContainer && (
                  <div className="kit-doc">
                    <div><div className="panel-x" style={{ marginBottom: 6 }}>RETHEME KIT</div><div className="retheme-sw">{['transparent', '#00bfff', '#ff2db5', '#fbbf24', '#39ff14', '#b06bff'].map(s => <button key={s} className={theme === s ? 'on' : ''} style={{ background: s === 'transparent' ? 'repeating-linear-gradient(45deg,#222 0 4px,#333 4px 8px)' : s }} onClick={() => setTheme(s)} />)}</div></div>
                    <div className="doc-card"><div className="dc-h">{selectedContainer.archetype}</div><div className="dc-b">This {selectedContainer.archetype.toLowerCase()} was built for {activeInst.name.split(' ')[0]}. Rename a control, change its range, recolor it, or rescript its formula — then it's yours to design with.</div><div className="dc-k">{selectedContainer.controls.length} CAPSULES · WIRE {selectedContainer.wire}</div></div>
                    <button className="addchip" style={{ alignSelf: 'flex-start' }} onClick={() => setDepth(2)}>⤒ SURFACE TO PATCH</button>
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* ROUTING — persistent */}
          <div className="panel" style={{ flex: '0 0 auto', padding: '7px 12px' }}>
            <div className="routebar">
              <span className="panel-t" style={{ flex: '0 0 auto' }}>Nexus Routing</span><span className="route-arrow">│</span>
              {layers.map((l, i) => (<React.Fragment key={l.id}><div className="route-node"><span className="route-jack on" style={{ boxShadow: l.visible ? `inset 0 1px 2px rgba(0,0,0,.8), 0 0 7px var(--${l.ch})` : 'inset 0 1px 2px rgba(0,0,0,.8)' }} /><span className="route-lbl">{l.name.split(' ')[0]}</span><span className="route-lbl" style={{ color: `var(--${l.ch})` }}>{l.blend}</span></div>{i < layers.length - 1 && <span className="route-arrow">→</span>}</React.Fragment>))}
              <span className="route-arrow">⇒</span>
              <div className="route-node" style={{ boxShadow: '0 0 12px var(--gd-g), var(--raised)' }}><span className="route-jack" style={{ boxShadow: 'inset 0 1px 2px rgba(0,0,0,.8), 0 0 7px var(--gd)' }} /><span className="route-lbl" style={{ color: 'var(--gd)' }}>BIOSPARK THEATER</span></div>
              <span className="grow" /><span className="route-lbl">SSoT · RUST · WS://AXIOM</span>
            </div>
          </div>
        </div>
      </div>

      {!booted && tw.boot !== false && <BootSplash onDone={() => setBooted(true)} />}

      {/* TWEAKS */}
      <TweaksPanel title="Tweaks">
        <TweakSection label="Chassis finish">
          <TweakSelect label="Metal" value={tw.finish} onChange={(v) => setTweak('finish', v)} options={[{ value: 'gunmetal', label: 'Gunmetal (default)' }, { value: 'alu', label: 'Brushed aluminum' }, { value: 'brass', label: 'Black + brass' }, { value: 'titan', label: 'Titanium + iridescent' }]} />
        </TweakSection>
        <TweakSection label="Theater glass">
          <TweakToggle label="Scanlines" value={tw.scanlines} onChange={(v) => setTweak('scanlines', v)} />
          <TweakToggle label="CRT vignette" value={tw.crt} onChange={(v) => setTweak('crt', v)} />
        </TweakSection>
        <TweakSection label="Boot">
          <TweakToggle label="Boot splash on load" value={tw.boot !== false} onChange={(v) => setTweak('boot', v)} />
          <TweakButton label="Replay boot sequence" onClick={() => setBooted(false)} />
        </TweakSection>
        <TweakSection label="Demo">
          <TweakButton label={playing ? 'Halt weave' : 'Weave time'} onClick={() => setPlaying(p => !p)} />
          <TweakButton label="Crash a ship" onClick={() => fire('SHIP')} />
          <TweakButton label="Dive to patch (L2)" onClick={() => setDepth(2)} />
        </TweakSection>
      </TweaksPanel>
    </div>
  );
}
ReactDOM.createRoot(document.getElementById('root')).render(<App />);
