/* global React */
// ═══════════════════════════════════════════════════════════════════
// RACK MODULES — the individual instruments
// ═══════════════════════════════════════════════════════════════════

const {
  Knob, Fader, Jack, SeqStep, Lissajous, Aurora, JogWheel, ResonanceMeter,
  Constellation, Lattice, MythosMap, HarmonicMeters, useLore, useSignal,
  GlyphSigil, GlyphHerald,
} = window;

// ── Helper: module header ────────────────────────────────────────────
function ModHeader({ id, name, motto, meta, channel, hover }) {
  return (
    <div className="module-header">
      <div className="module-name" {...hover}>
        <span className="num">{id}</span>
        <span>{name}</span>
      </div>
      {motto && <div className="module-motto">{motto}</div>}
      <div className="module-meta">
        <span className="led" />
        <span>{meta || 'ARMED'}</span>
      </div>
    </div>
  );
}

function Screws({ count = 2, bot }) {
  return (
    <div className={`module-screws ${bot ? 'bot' : ''}`}>
      {Array.from({ length: count }).map((_, i) => <div key={i} className="s" />)}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
// 01 · CONCORDANCE — master section
// ═══════════════════════════════════════════════════════════════════
function ConcordanceModule({ resonance, harmonics, illuminate, detachment, onIlluminate, onDetachment, lissajous }) {
  const lore = useLore.__shared;
  const onHover = {
    onMouseEnter: (e) => lore.show({ title: 'CONCORDANCE', glyph: '☉', body: 'The master section. Where all signals meet and the universe agrees with itself — or doesn\'t.', tag: 'HZ 432 · STRATUM 7' }, e),
    onMouseLeave: lore.hide,
  };

  return (
    <div className="module" data-channel="g" style={{ '--d-color': 'var(--gold)', '--d-glow': 'var(--gold-glow)' }}>
      <Screws />
      <ModHeader id="HCM/01" name="CONCORDANCE" motto={"As Above, So Below, So Woven."} meta={`φ ${(resonance * 432).toFixed(1)}HZ`} channel="g" hover={onHover}/>

      <div className="row" style={{ gap: 24 }}>
        {/* Left: orbit visualizer */}
        <div style={{ width: 220, height: 220, position: 'relative', flexShrink: 0 }}>
          <div className="crystal" style={{ width: 220, height: 220 }}>
            <Lissajous a={lissajous.a} b={lissajous.b} delta={lissajous.delta} size={220} color="var(--gold)" secondary="var(--astral-cyan)"/>
            {/* Center pip */}
            <div style={{ position: 'absolute', left: '50%', top: '50%', transform: 'translate(-50%,-50%)', width: 14, height: 14, borderRadius: 999, background: 'var(--gold)', boxShadow: '0 0 18px var(--gold-glow), 0 0 6px #fff' }}/>
            <div style={{ position: 'absolute', inset: 0, borderRadius: 999, border: '1px dashed rgba(251,191,36,0.3)', animation: 'spin-slow 28s linear infinite' }}/>
          </div>
          <div style={{ position: 'absolute', bottom: -4, left: '50%', transform: 'translateX(-50%)', fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.22em', color: 'var(--gold)', textTransform: 'uppercase' }}>
            ASTRAL GATEWAY
          </div>
        </div>

        {/* Center: portrait / vortex */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 14 }}>
          <div className="scope" style={{ height: 220 }}>
            {/* Vortex output */}
            <svg viewBox="0 0 400 220" preserveAspectRatio="none" style={{ width: '100%', height: '100%' }}>
              <defs>
                <radialGradient id="vortex" cx="50%" cy="50%" r="50%">
                  <stop offset="0%" stopColor="#fff" stopOpacity="0.9"/>
                  <stop offset="15%" stopColor="#fbbf24" stopOpacity="0.7"/>
                  <stop offset="40%" stopColor="#ff1493" stopOpacity="0.5"/>
                  <stop offset="70%" stopColor="#9400d3" stopOpacity="0.4"/>
                  <stop offset="100%" stopColor="#03050a" stopOpacity="0"/>
                </radialGradient>
                <filter id="vortex-glow"><feGaussianBlur stdDeviation="6"/></filter>
              </defs>
              <ellipse cx="200" cy="110" rx="180" ry="90" fill="url(#vortex)" opacity="0.85"/>
              {[...Array(12)].map((_, i) => {
                const r = 30 + i * 8;
                return (
                  <ellipse key={i} cx="200" cy="110" rx={r * 1.6} ry={r * 0.8}
                    fill="none" stroke="rgba(0,191,255,0.18)" strokeWidth="0.6"
                    transform={`rotate(${i * 8} 200 110)`}/>
                );
              })}
              {/* central singularity */}
              <circle cx="200" cy="110" r="10" fill="#fff"/>
              <circle cx="200" cy="110" r="20" fill="none" stroke="#fbbf24" strokeWidth="1" opacity="0.7" filter="url(#vortex-glow)"/>
              <text x="20" y="30" fill="rgba(251,191,36,0.7)" fontFamily="var(--font-code)" fontSize="9" letterSpacing="3">FINAL.OUTPUT</text>
              <text x="20" y="200" fill="rgba(0,191,255,0.6)" fontFamily="var(--font-code)" fontSize="8" letterSpacing="2">/realities/this/now</text>
              <text x="320" y="30" fill="rgba(192,132,252,0.7)" fontFamily="var(--font-code)" fontSize="9" letterSpacing="3" textAnchor="end">{(resonance * 432).toFixed(2)} HZ</text>
            </svg>
          </div>
        </div>

        {/* Right: master controls */}
        <div style={{ width: 320, flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 12 }}>
          <ResonanceMeter value={resonance} harmonics={harmonics}/>

          <div style={{ display: 'flex', gap: 12, alignItems: 'flex-end', justifyContent: 'space-between' }}>
            <Knob label="ILLUMINATE" sublabel="GLOBAL GAIN" value={illuminate} onChange={onIlluminate} channel="q" size="sm"/>
            <Knob label="DETACHMENT" sublabel="OBSERVER Δ" value={detachment} onChange={onDetachment} channel="m" size="sm"/>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              <Jack id="conc.out.master" kind="out" label="MASTER" channel="q"/>
              <Jack id="conc.out.sync" kind="out" label="SYNC" channel="q"/>
              <Jack id="conc.in.sidechain" kind="in" label="S.CHAIN" channel="b"/>
            </div>
          </div>
        </div>
      </div>
      <Screws bot />
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
// 02 · AXIOM CARVER — knobs + Lissajous
// ═══════════════════════════════════════════════════════════════════
function AxiomCarverModule({ knobs, setKnob }) {
  const lore = useLore.__shared;
  const onHover = {
    onMouseEnter: (e) => lore.show({ title: 'AXIOM CARVER', glyph: '⚶', body: 'The Forge does not invent constants; it listens for the one already inside the thing. Tune carefully — downstream phenomena depend on it.', tag: 'NODE/AXM/rt · stratum: 0' }, e),
    onMouseLeave: lore.hide,
  };

  // Per-axiom sub-modules — mini knobs reference the Genesis registry pattern
  // (PRR / VLR / CCH / HSH = Pre-Registrar, Validator, Cache, Hash Tracker)
  const [sub, setSub] = React.useState({
    gravity:  [0.5, 0.6, 0.3, 0.7],
    causal:   [0.7, 0.5, 0.4, 0.6],
    entropy:  [0.3, 0.4, 0.5, 0.3],
    phase:    [0.6, 0.5, 0.55, 0.4],
  });
  const setSubVal = (axiom, idx, v) => setSub(s => ({ ...s, [axiom]: s[axiom].map((x, i) => i === idx ? v : x) }));

  const [pads, setPads] = React.useState({
    gravity: { M: false, S: false, A: true },
    causal:  { M: false, S: false, A: true },
    entropy: { M: false, S: false, A: true },
    phase:   { M: false, S: false, A: true },
  });
  const togglePad = (ax, p) => setPads(prev => ({ ...prev, [ax]: { ...prev[ax], [p]: !prev[ax][p] }}));

  // The 16 Standard Components for Axiom Carver (synthesized from the registry pattern)
  const stdComponents = [
    { symbol: 'WOA', name: 'World Origin Anchor', wire: 'SPA' },
    { symbol: 'HGM', name: 'Hex Grid Mapper', wire: 'SPA' },
    { symbol: 'SDC', name: 'Strata Depth Calc', wire: 'SPA' },
    { symbol: 'CTE', name: 'Coord Transform', wire: 'SPA' },
    { symbol: 'BEP', name: 'Bool Expression Parser', wire: 'LGC' },
    { symbol: 'CCB', name: 'Constraint Builder', wire: 'LGC' },
    { symbol: 'PDE', name: 'Predicate Evaluator', wire: 'LGC' },
    { symbol: 'LOC', name: 'Logical Op Chain', wire: 'LGC' },
    { symbol: 'WTC', name: 'Wire Type Checker', wire: 'LGC' },
    { symbol: 'SIV', name: 'Schema Integrity', wire: 'LGC' },
    { symbol: 'CTV', name: 'Covenant Validator', wire: 'LGC' },
    { symbol: 'CLE', name: 'Capacity Law', wire: 'LGC' },
    { symbol: 'REL', name: 'Rule Execution Log', wire: 'DAT' },
    { symbol: 'VFR', name: 'Validation Reporter', wire: 'DAT' },
    { symbol: 'DTR', name: 'Decision Trace', wire: 'NAR' },
    { symbol: 'LSP', name: 'Logic State Pub', wire: 'EVT' },
  ];
  const [activeStd, setActiveStd] = React.useState(0);

  // Channel strip definitions
  const strips = [
    { id: 'gravity', name: 'GRAVITY',  sub: '9.81 m/s²',    wire: 'SPA', value: knobs.gravity, onChange: v => setKnob('gravity', v),
      subKnobs: ['ANC', 'WGT', 'FLD', 'TIDE'] },
    { id: 'causal',  name: 'CAUSAL',   sub: 'FIDELITY',     wire: 'LGC', value: knobs.causal,  onChange: v => setKnob('causal',  v),
      subKnobs: ['LNK', 'PRP', 'CHN', 'BRH'] },
    { id: 'entropy', name: 'ENTROPY',  sub: 'DECAY',        wire: 'DAT', value: knobs.entropy, onChange: v => setKnob('entropy', v),
      subKnobs: ['RATE', 'HEAT', 'DST', 'DRFT'] },
    { id: 'phase',   name: 'PHASE',    sub: 'DIMENSIONAL',  wire: 'TMP', value: knobs.phase,   onChange: v => setKnob('phase',   v),
      subKnobs: ['SHFT', 'ROT', 'OFFS', 'LOCK'] },
  ];

  return (
    <div className="module" data-channel="q">
      <Screws/>
      <ModHeader id="MYTH-13/AXM" name="AXIOM CARVER · LOGIC" motto={"From constant, consequence."} meta="LIVE · 16STD" channel="q" hover={onHover}/>

      <div className="row" style={{ gap: 12, alignItems: 'stretch' }}>
        {/* Channel strips */}
        <div style={{ display: 'flex', gap: 4 }}>
          {strips.map(s => {
            const meter = s.value * 0.6 + sub[s.id].reduce((a, b) => a + b, 0) / sub[s.id].length * 0.4;
            return (
              <div key={s.id} className="channel-strip" style={{ '--d-color': `var(--wire-${s.wire})`, '--d-glow': `var(--wire-${s.wire})`, width: 76 }}>
                <div className="cs-head">{s.name}</div>
                <div className="cs-sub">{s.sub}</div>

                {/* Main macro knob */}
                <Knob value={s.value} onChange={s.onChange} size="md" compact valueFormatter={() => Math.round(s.value * 99).toString().padStart(2, '0')}/>

                {/* 4 sub-knobs in 2x2 */}
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 5, marginTop: 2 }}>
                  {s.subKnobs.map((lbl, i) => (
                    <div key={i} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 1 }}>
                      <Knob value={sub[s.id][i]} onChange={v => setSubVal(s.id, i, v)} size="xs" compact valueFormatter={() => ''}/>
                      <div style={{ fontFamily: 'var(--font-code)', fontSize: 6, letterSpacing: '0.06em', color: 'var(--fg-muted)' }}>{lbl}</div>
                    </div>
                  ))}
                </div>

                {/* Pads */}
                <div className="cs-row" style={{ marginTop: 2 }}>
                  <div className={`cs-pad ${pads[s.id].M ? 'on' : ''}`} onClick={() => togglePad(s.id, 'M')}>M</div>
                  <div className={`cs-pad ${pads[s.id].S ? 'on' : ''}`} onClick={() => togglePad(s.id, 'S')}>S</div>
                  <div className={`cs-pad ${pads[s.id].A ? 'on' : ''}`} onClick={() => togglePad(s.id, 'A')}>A</div>
                </div>

                {/* Meter + mini fader */}
                <div style={{ display: 'flex', gap: 4, alignItems: 'flex-end', justifyContent: 'center', marginTop: 2 }}>
                  <div className="cs-meter" style={{ height: 56 }}>
                    <div className="fill" style={{ height: `${Math.max(2, meter * 100)}%` }}/>
                    <div className="seg-marks"/>
                  </div>
                  <Fader value={s.value} onChange={s.onChange} height={72}/>
                </div>

                {/* Wire chip + jack */}
                <div style={{ display: 'flex', flexDirection: 'column', gap: 4, alignItems: 'center', paddingTop: 2, borderTop: '1px solid rgba(120,180,255,0.08)', width: '100%' }}>
                  <span className="wire-pip" style={{ '--wire': `var(--wire-${s.wire})` }}>{s.wire}</span>
                  <div style={{ display: 'flex', gap: 4 }}>
                    <Jack id={`axm.out.${s.id}`} kind="out" channel="q"/>
                    <Jack id={`axm.in.${s.id}`}  kind="in"  channel="q"/>
                  </div>
                </div>
              </div>
            );
          })}
        </div>

        {/* Big Lissajous in crystal */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8 }}>
          <div style={{ width: 232, height: 232, position: 'relative' }}>
            <div className="crystal" style={{ width: 232, height: 232 }}>
              <Lissajous a={knobs.gravity} b={knobs.causal} delta={knobs.phase} size={232}/>
              <div style={{ position: 'absolute', inset: '50% 0 0 0', height: 1, background: 'linear-gradient(90deg, transparent, rgba(0,191,255,0.2), transparent)' }}/>
              <div style={{ position: 'absolute', inset: '0 50% 0 0', width: 1, background: 'linear-gradient(180deg, transparent, rgba(0,191,255,0.2), transparent)' }}/>
            </div>
            {/* Corner readouts */}
            <div style={{ position: 'absolute', top: 4, left: 4, fontFamily: 'var(--font-code)', fontSize: 7, color: 'var(--astral-cyan)', letterSpacing: '0.15em' }}>
              a:{(1 + knobs.gravity * 6).toFixed(2)}
            </div>
            <div style={{ position: 'absolute', top: 4, right: 4, fontFamily: 'var(--font-code)', fontSize: 7, color: 'var(--astral-magenta)', letterSpacing: '0.15em' }}>
              b:{(1 + knobs.causal * 6).toFixed(2)}
            </div>
            <div style={{ position: 'absolute', bottom: 4, left: 4, fontFamily: 'var(--font-code)', fontSize: 7, color: 'var(--gold)', letterSpacing: '0.15em' }}>
              δ:{(knobs.phase * 360).toFixed(0)}°
            </div>
            <div style={{ position: 'absolute', bottom: 4, right: 4, fontFamily: 'var(--font-code)', fontSize: 7, color: 'var(--bio)', letterSpacing: '0.15em' }}>
              LIVE
            </div>
          </div>
          <div style={{ fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.22em', color: 'var(--fg-3)', textTransform: 'uppercase' }}>
            AXIOM WAVEFORM · Q{Math.floor(knobs.gravity * 16)}/16
          </div>
        </div>

        {/* Right column: Std Component grid + jacks */}
        <div style={{ width: 152, flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 8 }}>
          <div style={{ fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.22em', color: 'var(--fg-3)', textTransform: 'uppercase', textAlign: 'center' }}>
            16 STD · LOGIC
          </div>
          <StdComponentGrid components={stdComponents} activeIdx={activeStd} onSelect={setActiveStd}/>
          <div style={{ padding: '4px 6px', background: 'rgba(0,0,0,0.35)', border: '1px solid rgba(120,180,255,0.08)', borderRadius: 2, minHeight: 32 }}>
            <div style={{ fontFamily: 'var(--font-code)', fontSize: 7, letterSpacing: '0.18em', color: `var(--wire-${stdComponents[activeStd].wire})`, marginBottom: 2 }}>
              {stdComponents[activeStd].symbol} · {stdComponents[activeStd].wire}
            </div>
            <div style={{ fontFamily: 'var(--font-script)', fontStyle: 'italic', fontSize: 9, color: 'var(--fg-2)' }}>
              {stdComponents[activeStd].name}
            </div>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 6, paddingTop: 4 }}>
            <Jack id="axm.cv.mod"   kind="cv" label="MOD"  channel="m"/>
            <Jack id="axm.cv.trig"  kind="cv" label="TRIG" channel="m"/>
            <Jack id="axm.cv.sync"  kind="cv" label="SYNC" channel="m"/>
            <Jack id="axm.out.bus"  kind="out" label="BUS"  channel="q"/>
            <Jack id="axm.out.aux"  kind="out" label="AUX"  channel="q"/>
            <Jack id="axm.in.fb"    kind="in"  label="FB"   channel="b"/>
          </div>
        </div>
      </div>

      {/* Telemetry strip */}
      <div style={{ marginTop: 10, height: 24, background: 'var(--panel-screen)', borderRadius: 3, padding: '0 12px', display: 'flex', alignItems: 'center', justifyContent: 'space-between', border: '1px solid rgba(0,229,255,0.12)' }}>
        <span style={{ fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.2em', color: 'var(--bio)' }}>
          ⚷ ETCHED · 16 AXIOMS · 256 CAPSULES
        </span>
        <span style={{ fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.2em', color: 'var(--astral-cyan)' }}>
          BSQM-001 · MYTH-13/AXM · {(knobs.gravity * 100 + knobs.causal * 50).toFixed(0)}HZ
        </span>
        <span style={{ fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.2em', color: 'var(--gold)' }}>
          ✓ SEALED
        </span>
      </div>
      <Screws bot/>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
// 03 · PERSONA FORGER — faders + aurora
// ═══════════════════════════════════════════════════════════════════
function PersonaForgerModule({ faders, setFader, lfos, setLfo }) {
  const lore = useLore.__shared;
  const onHover = {
    onMouseEnter: (e) => lore.show({ title: 'PERSONA FORGER', glyph: '☿', body: 'Five archetypes, three motive LFOs, one voice. Slide a fader and a sentience shifts.', tag: 'NODE/PRSF/rt · capacity ≤ 16' }, e),
    onMouseLeave: lore.hide,
  };

  const archs = [
    { id: 'hero',      label: 'HERO',      wire: 'IDN', sub: 'A1' },
    { id: 'trickster', label: 'TRICKSTER', wire: 'BHV', sub: 'A2' },
    { id: 'sage',      label: 'SAGE',      wire: 'NAR', sub: 'A3' },
    { id: 'shadow',    label: 'SHADOW',    wire: 'ENR', sub: 'A4' },
    { id: 'anima',     label: 'ANIMA',     wire: 'DAT', sub: 'A5' },
  ];

  const [strip, setStrip] = React.useState(
    Object.fromEntries(archs.map(a => [a.id, { soul: 0.5, form: 0.5, voice: 0.5, aux1: 0.3, aux2: 0.4, M: false, S: false, R: true }]))
  );
  const setStripVal = (id, k, v) => setStrip(prev => ({ ...prev, [id]: { ...prev[id], [k]: v }}));
  const tog = (id, k) => setStrip(prev => ({ ...prev, [id]: { ...prev[id], [k]: !prev[id][k] }}));

  const stdComponents = [
    { symbol: 'SWI', name: 'Soul Weight Init',     wire: 'IDN' },
    { symbol: 'EAC', name: 'Emotion Array',        wire: 'BHV' },
    { symbol: 'FSC', name: 'Fear Stat Calibrator', wire: 'BHV' },
    { symbol: 'DPR', name: 'Drive Priority',       wire: 'BHV' },
    { symbol: 'RTS', name: 'Race Template',        wire: 'DAT' },
    { symbol: 'BDG', name: 'B-DNA Generator',      wire: 'IDN' },
    { symbol: 'PPS', name: 'Physiology Params',    wire: 'DAT' },
    { symbol: 'GMB', name: 'GLTF Model Binder',    wire: 'AST' },
    { symbol: 'ASM', name: 'Animation State',      wire: 'BHV' },
    { symbol: 'RCM', name: 'Rig Control Mapper',   wire: 'DAT' },
    { symbol: 'GLR', name: 'Gesture Library',      wire: 'BHV' },
    { symbol: 'EBT', name: 'Expression Blend',     wire: 'VIS' },
    { symbol: 'LHW', name: 'Lineage Hash Writer',  wire: 'IDN' },
    { symbol: 'FAE', name: 'Faction Affiliation',  wire: 'SOC' },
    { symbol: 'MSI', name: 'Memory Seed Inject',   wire: 'NAR' },
    { symbol: 'ICS', name: 'Identity Capsule',     wire: 'IDN' },
  ];
  const [activeStd, setActiveStd] = React.useState(0);

  return (
    <div className="module" data-channel="m">
      <Screws/>
      <ModHeader id="MYTH-05/PRSF" name="PERSONA FORGER · ANIMUS" motto={"From mark, mandate."} meta="16 STD · IDN" channel="m" hover={onHover}/>

      <div className="row" style={{ gap: 12, alignItems: 'stretch' }}>
        {/* 5 Channel Strips */}
        <div style={{ display: 'flex', gap: 4 }}>
          {archs.map((a, i) => {
            const v = faders[i];
            const s = strip[a.id];
            const wireCol = `var(--wire-${a.wire})`;
            const meter = v * 0.5 + (s.soul + s.form + s.voice) / 3 * 0.5;
            return (
              <div key={a.id} className="channel-strip" style={{ '--d-color': wireCol, '--d-glow': wireCol, width: 70 }}>
                <div className="cs-head">{a.label}</div>
                <div className="cs-sub">{a.sub} · {a.wire}</div>

                {/* EQ section — Soul / Form / Voice */}
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 3, paddingTop: 2 }}>
                  {[
                    { k: 'soul',  l: 'SOL' },
                    { k: 'form',  l: 'FRM' },
                    { k: 'voice', l: 'VOC' },
                  ].map(eq => (
                    <div key={eq.k} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 1 }}>
                      <Knob value={s[eq.k]} onChange={v => setStripVal(a.id, eq.k, v)} size="xs" compact valueFormatter={() => ''}/>
                      <div style={{ fontFamily: 'var(--font-code)', fontSize: 5.5, letterSpacing: '0.05em', color: 'var(--fg-muted)' }}>{eq.l}</div>
                    </div>
                  ))}
                </div>

                {/* AUX sends */}
                <div style={{ display: 'flex', gap: 6, paddingTop: 2 }}>
                  <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 1 }}>
                    <Knob value={s.aux1} onChange={v => setStripVal(a.id, 'aux1', v)} size="xs" compact valueFormatter={() => ''}/>
                    <div style={{ fontFamily: 'var(--font-code)', fontSize: 5.5, color: 'var(--fg-muted)' }}>AX1</div>
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 1 }}>
                    <Knob value={s.aux2} onChange={v => setStripVal(a.id, 'aux2', v)} size="xs" compact valueFormatter={() => ''}/>
                    <div style={{ fontFamily: 'var(--font-code)', fontSize: 5.5, color: 'var(--fg-muted)' }}>AX2</div>
                  </div>
                </div>

                {/* M / S / R pads */}
                <div className="cs-row" style={{ marginTop: 2 }}>
                  <div className={`cs-pad ${s.M ? 'on' : ''}`} onClick={() => tog(a.id, 'M')}>M</div>
                  <div className={`cs-pad ${s.S ? 'on' : ''}`} onClick={() => tog(a.id, 'S')}>S</div>
                  <div className={`cs-pad ${s.R ? 'on' : ''}`} onClick={() => tog(a.id, 'R')}>R</div>
                </div>

                {/* Meter + fader */}
                <div style={{ display: 'flex', gap: 4, alignItems: 'flex-end', justifyContent: 'center', flex: 1, marginTop: 2 }}>
                  <div className="cs-meter" style={{ height: 92 }}>
                    <div className="fill" style={{ height: `${Math.max(2, meter * 100)}%`, background: `linear-gradient(180deg, var(--ember), ${wireCol})` }}/>
                    <div className="seg-marks"/>
                  </div>
                  <Fader value={v} onChange={vv => setFader(i, vv)} height={108}/>
                </div>

                {/* Wire chip + jacks */}
                <div style={{ display: 'flex', flexDirection: 'column', gap: 4, alignItems: 'center', paddingTop: 2, borderTop: '1px solid rgba(120,180,255,0.08)', width: '100%' }}>
                  <span className="wire-pip" style={{ '--wire': wireCol }}>{a.wire}</span>
                  <div style={{ display: 'flex', gap: 4 }}>
                    <Jack id={`prsf.out.${a.id}`} kind="out" channel="m"/>
                    <Jack id={`prsf.in.${a.id}`}  kind="in"  channel="b"/>
                  </div>
                </div>
              </div>
            );
          })}
        </div>

        {/* Aurora + LFOs */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 8 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
            <span style={{ fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.22em', color: 'var(--fg-3)' }}>PSYCHE-SPECTRUM · LIVE</span>
            <span style={{ fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.18em', color: 'var(--astral-magenta)' }}>
              {archs.map((a, i) => `${a.label[0]}${Math.round(faders[i] * 9)}`).join('·')}
            </span>
          </div>
          <div className="aurora" style={{ height: 196 }}>
            <Aurora values={faders}/>
            <div style={{ position: 'absolute', inset: 0, pointerEvents: 'none', background: 'repeating-linear-gradient(90deg, transparent 0 20%, rgba(192,132,252,0.06) 20%, transparent 20.5%)' }}/>
          </div>

          {/* LFO row + vocalic + B-DNA */}
          <div style={{ display: 'flex', gap: 10, alignItems: 'flex-end', justifyContent: 'space-between', padding: '6px 8px', background: 'rgba(0,0,0,0.3)', borderRadius: 3, border: '1px solid rgba(120,180,255,0.08)' }}>
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: 4 }}>
              <span style={{ fontFamily: 'var(--font-code)', fontSize: 7, letterSpacing: '0.2em', color: 'var(--fg-3)' }}>MOTIVE LFO</span>
              <div style={{ display: 'flex', gap: 8 }}>
                <Knob label="AMB" value={lfos.ambition} onChange={v => setLfo('ambition', v)} channel="g" size="sm"/>
                <Knob label="SUR" value={lfos.survival} onChange={v => setLfo('survival', v)} channel="r" size="sm"/>
                <Knob label="CUR" value={lfos.curio}    onChange={v => setLfo('curio',    v)} channel="q" size="sm"/>
              </div>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: 4 }}>
              <span style={{ fontFamily: 'var(--font-code)', fontSize: 7, letterSpacing: '0.2em', color: 'var(--fg-3)' }}>VOCALIC FILTER</span>
              <div style={{ display: 'flex', gap: 8 }}>
                <Knob label="CUT" value={0.7}  onChange={() => {}} channel="m" size="sm"/>
                <Knob label="RES" value={0.4}  onChange={() => {}} channel="m" size="sm"/>
                <Knob label="DRV" value={0.55} onChange={() => {}} channel="m" size="sm"/>
                <Knob label="TIM" value={0.6}  onChange={() => {}} channel="m" size="sm"/>
              </div>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: 4 }}>
              <span style={{ fontFamily: 'var(--font-code)', fontSize: 7, letterSpacing: '0.2em', color: 'var(--fg-3)' }}>B-DNA · μ 0.0006</span>
              <div style={{ fontFamily: 'var(--font-code)', fontSize: 10, letterSpacing: '0.05em', color: 'var(--bio)', background: 'rgba(0,0,0,0.6)', padding: '5px 8px', border: '1px solid rgba(57,255,20,0.2)', borderRadius: 2 }}>
                0x{faders.map(f => Math.round(f * 15).toString(16).toUpperCase()).join('')}
              </div>
              <div style={{ fontFamily: 'var(--font-code)', fontSize: 6.5, letterSpacing: '0.18em', color: 'var(--fg-muted)' }}>LINEAGE · ANIMUS</div>
            </div>
          </div>
        </div>

        {/* Std Components grid */}
        <div style={{ width: 152, flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 8 }}>
          <div style={{ fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.22em', color: 'var(--fg-3)', textTransform: 'uppercase', textAlign: 'center' }}>
            16 STD · ANIMUS
          </div>
          <StdComponentGrid components={stdComponents} activeIdx={activeStd} onSelect={setActiveStd}/>
          <div style={{ padding: '4px 6px', background: 'rgba(0,0,0,0.35)', border: '1px solid rgba(120,180,255,0.08)', borderRadius: 2, minHeight: 32 }}>
            <div style={{ fontFamily: 'var(--font-code)', fontSize: 7, letterSpacing: '0.18em', color: `var(--wire-${stdComponents[activeStd].wire})`, marginBottom: 2 }}>
              {stdComponents[activeStd].symbol} · {stdComponents[activeStd].wire}
            </div>
            <div style={{ fontFamily: 'var(--font-script)', fontStyle: 'italic', fontSize: 9, color: 'var(--fg-2)' }}>
              {stdComponents[activeStd].name}
            </div>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 6, paddingTop: 4 }}>
            <Jack id="prsf.cv.psyche" kind="cv" label="PSY"  channel="m"/>
            <Jack id="prsf.cv.shadow" kind="cv" label="SHA"  channel="m"/>
            <Jack id="prsf.cv.dna"    kind="cv" label="DNA"  channel="m"/>
            <Jack id="prsf.out.anima" kind="out" label="ANI" channel="m"/>
            <Jack id="prsf.out.voice" kind="out" label="VOX" channel="m"/>
            <Jack id="prsf.in.motive" kind="in"  label="MOT" channel="b"/>
          </div>
        </div>
      </div>
      <Screws bot/>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
// 04 · CHRONOFLOW SEQUENCER — steps + jog wheel
// ═══════════════════════════════════════════════════════════════════
function ChronoFlowModule({ steps, toggleStep, playhead, playing, onPlay, jog, setJog, tempo, setTempo, swing, setSwing }) {
  const lore = useLore.__shared;
  const onHover = {
    onMouseEnter: (e) => lore.show({ title: 'CHRONOFLOW SEQUENCER', glyph: '☾', body: '16 nexus events. Each step is a moment that may or may not happen. Scrub the wheel to walk the timeline.', tag: 'NODE/CHRN/rt · 16STP' }, e),
    onMouseLeave: lore.hide,
  };

  const eventNames = ['ORIGIN', 'CALL', 'THRSHLD', 'TRIAL', 'MENTOR', 'ABYSS', 'BOON', 'RETURN', 'MASTER', 'SACRIFICE', 'REBIRTH', 'APOTHEOSIS', 'COUNCIL', 'AWARD', 'TRANSCEND', 'CODA'];

  return (
    <div className="module" data-channel="b">
      <Screws/>
      <ModHeader id="CHRN/04" name="CHRONOFLOW SEQUENCER" motto={"Each tick a fate."} meta={playing ? '▶ PLAYING' : '◼ STOPPED'} channel="b" hover={onHover}/>

      <div className="row" style={{ gap: 18 }}>
        {/* Jog Wheel */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10, alignItems: 'center' }}>
          <JogWheel value={jog} onChange={setJog} epoch={`EPOCH ${Math.floor(((jog % 1) + 1) % 1 * 12) + 1}`}/>
          <div style={{ display: 'flex', gap: 8 }}>
            <button
              onClick={onPlay}
              style={{
                fontFamily: 'var(--font-header)', fontSize: 10, letterSpacing: '0.18em',
                padding: '6px 12px', borderRadius: 2,
                background: playing ? 'rgba(57,255,20,0.12)' : 'rgba(0,0,0,0.4)',
                border: `1px solid ${playing ? 'var(--bio)' : 'rgba(120,180,255,0.18)'}`,
                color: playing ? 'var(--bio)' : 'var(--fg-2)',
                cursor: 'pointer', textTransform: 'uppercase',
                boxShadow: playing ? '0 0 12px var(--bio-glow)' : 'none',
              }}
            >
              {playing ? '◼ Halt' : '▶ Weave'}
            </button>
            <button
              onClick={() => toggleStep('reset')}
              style={{ fontFamily: 'var(--font-header)', fontSize: 10, letterSpacing: '0.18em',
                padding: '6px 10px', borderRadius: 2,
                background: 'rgba(0,0,0,0.4)',
                border: '1px solid rgba(120,180,255,0.18)',
                color: 'var(--fg-3)', cursor: 'pointer', textTransform: 'uppercase' }}
            >
              ⟲ Clear
            </button>
          </div>
        </div>

        {/* Sequencer grid */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 10 }}>
          {/* Probability knobs row */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(16, 1fr)', gap: 6 }}>
            {steps.map((s, i) => (
              <div key={i} style={{ display: 'flex', justifyContent: 'center' }}>
                <div className="knob xs"
                  style={{
                    '--d-color': 'var(--astral-cyan)', '--d-glow': 'var(--astral-cyan-glow)',
                    opacity: s.on ? 1 : 0.4,
                  }}>
                  <div className="indicator" style={{ transform: `rotate(${-135 + s.prob * 270}deg)` }}/>
                </div>
              </div>
            ))}
          </div>

          {/* Steps */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(16, 1fr)', gap: 6 }}>
            {steps.map((s, i) => (
              <SeqStep key={i} idx={i} on={s.on} playing={playing && playhead === i} onToggle={toggleStep}/>
            ))}
          </div>

          {/* Event names */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(16, 1fr)', gap: 6 }}>
            {steps.map((s, i) => (
              <div key={i} style={{ fontFamily: 'var(--font-code)', fontSize: 6.5, letterSpacing: '0.1em', color: playhead === i && playing ? 'var(--ember)' : 'var(--fg-muted)', textTransform: 'uppercase', textAlign: 'center', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                {eventNames[i]}
              </div>
            ))}
          </div>

          {/* Timeline waveform */}
          <div className="scope" style={{ height: 48, padding: '4px 8px' }}>
            <svg viewBox="0 0 800 40" preserveAspectRatio="none" style={{ width: '100%', height: '100%', position: 'relative' }}>
              <polyline
                points={steps.map((s, i) => `${(i / 15) * 800},${s.on ? 8 + (1 - s.prob) * 24 : 28}`).join(' ')}
                fill="none" stroke="var(--astral-cyan)" strokeWidth="1.4" strokeOpacity="0.7"/>
              {steps.map((s, i) => (
                <circle key={i} cx={(i / 15) * 800} cy={s.on ? 8 + (1 - s.prob) * 24 : 28}
                  r={s.on ? 2.5 : 1.5}
                  fill={playhead === i && playing ? '#f97316' : 'var(--astral-cyan)'}
                  opacity={s.on ? 0.9 : 0.4}
                  filter={s.on ? 'drop-shadow(0 0 4px var(--astral-cyan))' : undefined}/>
              ))}
              {playing && (
                <line x1={(playhead / 15) * 800} y1="0" x2={(playhead / 15) * 800} y2="40"
                  stroke="var(--ember)" strokeWidth="1" strokeOpacity="0.7"/>
              )}
            </svg>
          </div>
        </div>

        {/* Tempo / Swing / Jacks */}
        <div style={{ width: 100, flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 14, alignItems: 'center' }}>
          <Knob label="TEMPO" sublabel={`${(60 + tempo * 180).toFixed(0)}BPM`} value={tempo} onChange={setTempo} channel="b" size="xs"/>
          <Knob label="SWING" sublabel="GROOVE" value={swing} onChange={setSwing} channel="m" size="xs"/>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2,1fr)', gap: 8 }}>
            <Jack id="chrn.out.clock" kind="out" label="CLK" channel="b"/>
            <Jack id="chrn.out.gate"  kind="out" label="GATE" channel="b"/>
            <Jack id="chrn.in.reset"  kind="in"  label="RST" channel="b"/>
            <Jack id="chrn.in.tempo"  kind="in"  label="TMP" channel="b"/>
          </div>
        </div>
      </div>
      <Screws bot/>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
// 05 · RESONANCE SEALER — harmonic meters
// ═══════════════════════════════════════════════════════════════════
function ResonanceSealerModule({ harmonics }) {
  const lore = useLore.__shared;
  const onHover = {
    onMouseEnter: (e) => lore.show({ title: 'RESONANCE SEALER', glyph: 'ᛟ', body: 'Twelve harmonic bands. When all sing on key, the reality holds. When they don\'t — re-forge from prime.', tag: 'NODE/RES/rt · 12BAND' }, e),
    onMouseLeave: lore.hide,
  };

  return (
    <div className="module" data-channel="b">
      <Screws count={2}/>
      <ModHeader id="RES/05" name="RES SEALER" channel="b" hover={onHover}/>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        <HarmonicMeters values={harmonics}/>
        <div style={{ display: 'flex', gap: 10, alignItems: 'center', justifyContent: 'space-between', paddingTop: 4 }}>
          <Knob value={harmonics[5]} onChange={() => {}} label="SEAL" channel="b" size="sm"/>
          <div style={{ flex: 1, padding: '4px 8px', background: 'rgba(0,0,0,0.4)', borderRadius: 2, border: '1px solid rgba(57,255,20,0.18)' }}>
            <div style={{ fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.18em', color: 'var(--bio)', marginBottom: 2 }}>VOW</div>
            <div style={{ fontFamily: 'var(--font-script)', fontStyle: 'italic', fontSize: 10, color: 'var(--fg-2)' }}>
              "The seal holds while the song holds."
            </div>
          </div>
        </div>
        <div style={{ display: 'flex', gap: 6, justifyContent: 'space-between', paddingTop: 2 }}>
          <Jack id="res.in.feed" kind="in" label="FEED" channel="b"/>
          <Jack id="res.in.cv"   kind="cv" label="CV"   channel="m"/>
          <Jack id="res.out.fund" kind="out" label="FUND" channel="b"/>
          <Jack id="res.out.7th"  kind="out" label="7TH"  channel="b"/>
        </div>
      </div>
      <Screws bot count={2}/>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
// 06 · MYTHOS CARTOGRAPHER — narrative map
// ═══════════════════════════════════════════════════════════════════
function MythosCartographerModule({ activeArc, setActiveArc }) {
  const lore = useLore.__shared;
  const arcs = ['origin', 'descent', 'apotheosis'];
  const onHover = {
    onMouseEnter: (e) => lore.show({ title: 'MYTHOS CARTOGRAPHER', glyph: '𓂀', body: 'Three arcs, twelve waypoints. Every story is already mapped — you only choose which path to illuminate.', tag: 'NODE/MYTH/rt' }, e),
    onMouseLeave: lore.hide,
  };

  return (
    <div className="module" data-channel="m">
      <Screws count={2}/>
      <ModHeader id="MYTH/06" name="MYTHOS MAP" channel="m" hover={onHover}/>
      <div className="scope" style={{ height: 156 }}>
        <MythosMap activeArc={activeArc}/>
      </div>
      <div style={{ display: 'flex', gap: 4, marginTop: 8 }}>
        {arcs.map(a => (
          <button
            key={a}
            onClick={() => setActiveArc(a)}
            style={{
              flex: 1,
              fontFamily: 'var(--font-header)', fontSize: 9, letterSpacing: '0.2em', textTransform: 'uppercase',
              padding: '5px 0', borderRadius: 2,
              background: activeArc === a ? 'rgba(192,132,252,0.12)' : 'rgba(0,0,0,0.3)',
              border: `1px solid ${activeArc === a ? 'rgba(192,132,252,0.5)' : 'rgba(120,180,255,0.12)'}`,
              color: activeArc === a ? 'var(--mythos)' : 'var(--fg-3)',
              cursor: 'pointer',
              boxShadow: activeArc === a ? '0 0 8px var(--mythos-glow)' : 'none',
            }}
          >
            {a}
          </button>
        ))}
      </div>
      <div style={{ display: 'flex', gap: 6, justifyContent: 'space-between', paddingTop: 8 }}>
        <Jack id="myth.in.arc" kind="in" label="ARC" channel="b"/>
        <Jack id="myth.in.beat" kind="cv" label="BEAT" channel="m"/>
        <Jack id="myth.out.fate" kind="out" label="FATE" channel="m"/>
        <Jack id="myth.out.echo" kind="out" label="ECHO" channel="m"/>
      </div>
      <Screws bot count={2}/>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
// 07 · VOID LATTICE ENGINEER — stress heatmap
// ═══════════════════════════════════════════════════════════════════
function VoidLatticeModule({ stress }) {
  const lore = useLore.__shared;
  const onHover = {
    onMouseEnter: (e) => lore.show({ title: 'VOID LATTICE ENGINEER', glyph: '⊟', body: 'Spacetime is woven; the lattice frays. Cyan = stable, gold = stressed, orange = imminent rupture.', tag: 'NODE/VLE/rt · 72CELL' }, e),
    onMouseLeave: lore.hide,
  };
  return (
    <div className="module" data-channel="g">
      <Screws count={2}/>
      <ModHeader id="VLE/07" name="VOID LATTICE" channel="g" hover={onHover}/>
      <div style={{ height: 156, background: 'var(--panel-screen)', borderRadius: 3, border: '1px solid rgba(57,255,20,0.12)', overflow: 'hidden' }}>
        <Lattice stress={stress}/>
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 8, fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.18em', color: 'var(--fg-3)' }}>
        <span>STRESS · {Math.round(stress * 100)}%</span>
        <span style={{ color: stress > 0.6 ? 'var(--ember)' : stress > 0.4 ? 'var(--gold)' : 'var(--bio)' }}>
          {stress > 0.6 ? '⚠ RUPTURE' : stress > 0.4 ? '◈ TENSE' : '✓ STABLE'}
        </span>
      </div>
      <div style={{ display: 'flex', gap: 6, justifyContent: 'space-between', paddingTop: 8 }}>
        <Jack id="vle.in.gravity" kind="in" label="GRAV" channel="b"/>
        <Jack id="vle.in.weave"   kind="cv" label="WEAVE" channel="m"/>
        <Jack id="vle.out.stress" kind="out" label="STRS" channel="g"/>
        <Jack id="vle.out.repair" kind="out" label="REPR" channel="b"/>
      </div>
      <Screws bot count={2}/>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
// 08 · NEBULA SYSTEMS MONITOR — constellation
// ═══════════════════════════════════════════════════════════════════
function NebulaSystemsModule() {
  const lore = useLore.__shared;
  const onHover = {
    onMouseEnter: (e) => lore.show({ title: 'NEBULA SYSTEMS MONITOR', glyph: '✶', body: 'Each star is a server, each constellation a cluster. Errors pulse red; healthy nodes burn white-hot.', tag: 'NODE/NSM/rt · 3CLST' }, e),
    onMouseLeave: lore.hide,
  };
  const clusters = [
    { color: '#00bfff', pts: [[60, 60], [100, 40], [140, 70], [120, 110], [80, 120]], lines: [[0,1],[1,2],[2,3],[3,4],[4,0],[0,2]] },
    { color: '#ff1493', pts: [[260, 50], [300, 90], [340, 60], [380, 100], [320, 130]], lines: [[0,1],[1,2],[2,3],[3,4],[4,0]] },
    { color: '#fbbf24', pts: [[440, 80], [490, 50], [520, 110], [560, 80]], lines: [[0,1],[0,2],[1,3],[2,3]] },
  ];
  return (
    <div className="module" data-channel="q">
      <Screws count={2}/>
      <ModHeader id="NSM/08" name="NEBULA MONITOR" channel="q" hover={onHover}/>
      <div className="scope" style={{ height: 156, position: 'relative' }}>
        <Constellation nodes={clusters}/>
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 8, fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.18em' }}>
        <span style={{ color: 'var(--astral-cyan)' }}>◯ AETHER · OK</span>
        <span style={{ color: 'var(--astral-magenta)' }}>◯ PULSE · OK</span>
        <span style={{ color: 'var(--gold)' }}>◯ FORGE · WARN</span>
      </div>
      <div style={{ display: 'flex', gap: 6, justifyContent: 'space-between', paddingTop: 8 }}>
        <Jack id="nsm.in.feed"   kind="in"  label="FEED" channel="b"/>
        <Jack id="nsm.out.alert" kind="out" label="ALRT" channel="g"/>
        <Jack id="nsm.out.load"  kind="out" label="LOAD" channel="q"/>
        <Jack id="nsm.out.sync"  kind="out" label="SYNC" channel="q"/>
      </div>
      <Screws bot count={2}/>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
// 09 · NEXUS PATCH BAY — connection matrix
// ═══════════════════════════════════════════════════════════════════
function NexusPatchBayModule({ cables, clearCables }) {
  const lore = useLore.__shared;
  const onHover = {
    onMouseEnter: (e) => lore.show({ title: 'NEXUS PATCH BAY', glyph: 'ᛚ', body: 'Click any output jack, then click any input. Click a patched jack again to sever it.', tag: 'NODE/NXS/rt · MATRIX' }, e),
    onMouseLeave: lore.hide,
  };

  // Generate labeled bay rows
  const rows = [
    { label: 'AXIOM',  ch: 'q', count: 8 },
    { label: 'PERSONA', ch: 'm', count: 8 },
    { label: 'CHRONO', ch: 'b', count: 8 },
    { label: 'MYTHOS', ch: 'm', count: 8 },
  ];

  return (
    <div className="module" data-channel="g" style={{ '--d-color': 'var(--gold)', '--d-glow': 'var(--gold-glow)' }}>
      <Screws/>
      <ModHeader id="NXS/09" name="NEXUS PATCH BAY" motto={"Bind, weave, sever."} meta={`${cables.length} BOUND`} channel="g" hover={onHover}/>

      <div className="row" style={{ gap: 18, alignItems: 'stretch' }}>
        {/* Patch bay matrix */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 8 }}>
          {rows.map((r, ri) => (
            <div key={ri} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <div style={{ width: 64, fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.22em', color: 'var(--fg-3)', textTransform: 'uppercase' }}>{r.label}</div>
              <div style={{ flex: 1, display: 'grid', gridTemplateColumns: `repeat(${r.count}, 1fr)`, gap: 8 }}>
                {Array.from({ length: r.count }).map((_, i) => (
                  <div key={i} style={{ display: 'flex', justifyContent: 'center' }}>
                    <Jack id={`nxs.${r.label.toLowerCase()}.${i}`} kind={i % 2 === 0 ? 'out' : 'in'} channel={r.ch}/>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>

        {/* Status panel */}
        <div style={{ width: 220, flexShrink: 0, padding: 12, background: 'rgba(0,0,0,0.4)', borderRadius: 3, border: '1px solid rgba(251,191,36,0.15)', display: 'flex', flexDirection: 'column', gap: 10 }}>
          <div style={{ fontFamily: 'var(--font-header)', fontSize: 10, letterSpacing: '0.28em', color: 'var(--gold)', textTransform: 'uppercase' }}>BINDINGS</div>
          <div style={{ flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 4, fontFamily: 'var(--font-code)', fontSize: 8, letterSpacing: '0.08em', color: 'var(--fg-2)', maxHeight: 140 }}>
            {cables.length === 0 && (
              <div style={{ color: 'var(--fg-muted)', fontStyle: 'italic', fontFamily: 'var(--font-script)', fontSize: 10 }}>No cables bound. The Forge waits.</div>
            )}
            {cables.map((c, i) => (
              <div key={i} style={{ display: 'flex', justifyContent: 'space-between', padding: '2px 0', borderBottom: '1px solid rgba(120,180,255,0.05)' }}>
                <span style={{ color: 'var(--astral-cyan)' }}>{c.from.split('.').slice(-2).join('.')}</span>
                <span style={{ color: 'var(--fg-muted)' }}>→</span>
                <span style={{ color: 'var(--bio)' }}>{c.to.split('.').slice(-2).join('.')}</span>
              </div>
            ))}
          </div>
          <button
            onClick={clearCables}
            style={{ fontFamily: 'var(--font-header)', fontSize: 9, letterSpacing: '0.22em', textTransform: 'uppercase',
              padding: '6px 10px', background: 'rgba(249,115,22,0.08)',
              border: '1px solid rgba(249,115,22,0.4)', color: 'var(--ember)',
              borderRadius: 2, cursor: 'pointer' }}
          >
            ⌀ SEVER ALL
          </button>
        </div>
      </div>
      <Screws bot/>
    </div>
  );
}

// ── Export ───────────────────────────────────────────────────────────
Object.assign(window, {
  ConcordanceModule, AxiomCarverModule, PersonaForgerModule,
  ChronoFlowModule, ResonanceSealerModule, MythosCartographerModule,
  VoidLatticeModule, NebulaSystemsModule, NexusPatchBayModule,
});
