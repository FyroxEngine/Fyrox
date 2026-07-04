/* global React, ReactDOM */
// ═══════════════════════════════════════════════════════════════════
// THE AXIOM SIGNAL CHAIN RACK · main app
// ═══════════════════════════════════════════════════════════════════
const { useState, useEffect, useMemo, useRef, useCallback } = React;
const {
  SignalContext, ConcordanceModule, AxiomCarverModule, PersonaForgerModule,
  ChronoFlowModule, ResonanceSealerModule, MythosCartographerModule,
  VoidLatticeModule, NebulaSystemsModule, NexusPatchBayModule,
  CablesLayer, useLore,
  GenesisMap, HierCrumb,
  TweaksPanel, TweakSection, TweakRadio, TweakSelect, TweakToggle, TweakSlider, TweakButton, useTweaks,
} = window;

function App() {
  // ── TWEAKS ─────────────────────────────────────────────────────────
  const [t, setTweak] = useTweaks(window.__RACK_TWEAKS);

  // Apply tweaks to body data-attrs for CSS hooks
  useEffect(() => {
    document.body.dataset.identity   = t.moduleIdentity;
    document.body.dataset.cable      = t.cableStyle;
    document.body.dataset.bg         = t.background;
    document.body.dataset.density    = t.density;
    document.body.dataset.intensity  = t.colorIntensity;
  }, [t]);

  // ── INSTRUMENT BANK GROUPS ─────────────────────────────────────────
  const [activeGroup, setActiveGroup] = useState('ALL');

  // ── AXIOM CARVER state ─────────────────────────────────────────────
  const [knobs, setKnobs] = useState({
    gravity: 0.5, causal: 0.6, entropy: 0.35, phase: 0.5,
  });
  const setKnob = (k, v) => setKnobs(prev => ({ ...prev, [k]: v }));

  // ── PERSONA FORGER state ───────────────────────────────────────────
  const [faders, setFaders] = useState([0.65, 0.4, 0.55, 0.3, 0.7]);
  const setFader = (i, v) => setFaders(prev => prev.map((f, idx) => idx === i ? v : f));
  const [lfos, setLfos] = useState({ ambition: 0.5, survival: 0.4, curio: 0.65 });
  const setLfo = (k, v) => setLfos(prev => ({ ...prev, [k]: v }));

  // ── CHRONOFLOW state ───────────────────────────────────────────────
  const [steps, setSteps] = useState(() => {
    const seed = [0,3,5,7,8,11,13,15];
    return Array.from({ length: 16 }, (_, i) => ({
      on: seed.includes(i),
      prob: 0.5 + (i % 4) * 0.12,
    }));
  });
  const [playhead, setPlayhead] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [tempo, setTempo] = useState(0.5);
  const [swing, setSwing] = useState(0.3);
  const [jog, setJog] = useState(0.25);

  const toggleStep = useCallback((i) => {
    if (i === 'reset') {
      setSteps(s => s.map(x => ({ ...x, on: false })));
      return;
    }
    setSteps(prev => prev.map((s, idx) => idx === i ? { ...s, on: !s.on } : s));
  }, []);

  // Sequencer clock
  useEffect(() => {
    if (!playing) return;
    const bpm = 60 + tempo * 180;
    const interval = (60 / bpm / 4) * 1000; // 16th notes
    const id = setInterval(() => {
      setPlayhead(p => (p + 1) % 16);
    }, interval);
    return () => clearInterval(id);
  }, [playing, tempo]);

  // ── MYTHOS state ───────────────────────────────────────────────────
  const [activeArc, setActiveArc] = useState('descent');

  // ── CONCORDANCE state ──────────────────────────────────────────────
  const [illuminate, setIlluminate] = useState(0.75);
  const [detachment, setDetachment] = useState(0.4);

  // ── CABLES / PATCHING ──────────────────────────────────────────────
  const [armedJack, setArmedJack] = useState(null);
  const [cables, setCables] = useState([
    { from: 'axm.out.gravity', to: 'vle.in.gravity' },
    { from: 'axm.out.causal',  to: 'myth.in.arc' },
    { from: 'prsf.out.anima',  to: 'conc.in.sidechain' },
  ]);
  const addCable = useCallback((c) => setCables(prev => [...prev, c]), []);
  const removeCableAt = useCallback((id) => {
    setCables(prev => prev.filter(c => c.from !== id && c.to !== id));
  }, []);
  const clearCables = useCallback(() => setCables([]), []);

  // Dismiss armed jack on outside click
  useEffect(() => {
    if (!armedJack) return;
    const handler = (e) => {
      if (!e.target.closest('.jack')) setArmedJack(null);
    };
    window.addEventListener('click', handler);
    return () => window.removeEventListener('click', handler);
  }, [armedJack]);

  // ── DERIVED: System Resonance ──────────────────────────────────────
  // Resonance is high when knob values are harmonious (close to simple ratios)
  // and patching is active. Faders also contribute.
  const resonance = useMemo(() => {
    const k = knobs;
    // Harmony score: distance from 0.5 and ratio neatness
    const harm = 1 - Math.abs(k.gravity - k.causal) * 0.5;
    const balance = 1 - Math.abs(k.entropy - 0.3) * 0.8;
    const personaAvg = faders.reduce((a, b) => a + b, 0) / faders.length;
    const cableBoost = Math.min(0.2, cables.length * 0.025);
    const seqDensity = steps.filter(s => s.on).length / 16;
    const score = (
      harm * 0.30 +
      balance * 0.20 +
      personaAvg * 0.18 +
      illuminate * 0.12 +
      seqDensity * 0.10 +
      cableBoost +
      (playing ? 0.05 : 0)
    );
    return Math.max(0, Math.min(1, score));
  }, [knobs, faders, illuminate, cables.length, steps, playing]);

  // 12-band harmonics derived from rack state
  const [harmTime, setHarmTime] = useState(0);
  useEffect(() => {
    let raf;
    const tick = () => { setHarmTime(performance.now() / 1000); raf = requestAnimationFrame(tick); };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);
  const harmonics = useMemo(() => {
    const t = harmTime;
    return Array.from({ length: 12 }, (_, i) => {
      const base = 0.2 + (Math.sin(t * (0.6 + i * 0.07) + i) * 0.5 + 0.5) * 0.5;
      const knobBias = (i < 4 ? Object.values(knobs)[i] : faders[i % 5]) || 0.5;
      const seqBias = steps[i % 16].on ? 0.2 : 0;
      const v = Math.max(0.05, Math.min(1, base * 0.5 + knobBias * 0.4 + seqBias + resonance * 0.1));
      return v;
    });
  }, [harmTime, knobs, faders, steps, resonance]);

  // Void Lattice stress (inverse of resonance, plus entropy boost)
  const stress = Math.max(0, Math.min(1, knobs.entropy * 0.7 + (1 - resonance) * 0.5));

  // Lissajous params for Concordance crystal
  const lissajous = { a: faders[0], b: knobs.causal, delta: knobs.phase };

  // ── LORE ───────────────────────────────────────────────────────────
  const lore = useLore();
  useLore.__shared = lore;

  // ── SIGNAL CONTEXT VALUE ───────────────────────────────────────────
  const signal = {
    armedJack, setArmedJack,
    cables, addCable, removeCableAt,
  };

  // ── MODULE GROUPS for show/hide ────────────────────────────────────
  const groups = ['ALL', 'AXIOM', 'PERSONA', 'CHRONO', 'INSTRUMENTS'];
  const inGroup = (g) => activeGroup === 'ALL' || activeGroup === g;

  return (
    <SignalContext.Provider value={signal}>
      <div className="page" style={{ position: 'relative' }}>
        {/* BRAND BAR */}
        <div className="brandbar">
          <div className="brand">
            <svg className="mark" viewBox="0 0 100 100">
              <defs>
                <linearGradient id="brand-grad" x1="0" y1="0" x2="1" y2="1">
                  <stop offset="0%" stopColor="#fbbf24"/>
                  <stop offset="50%" stopColor="#ff1493"/>
                  <stop offset="100%" stopColor="#00bfff"/>
                </linearGradient>
              </defs>
              <circle cx="50" cy="50" r="34" fill="none" stroke="url(#brand-grad)" strokeWidth="2"/>
              <circle cx="50" cy="50" r="22" fill="none" stroke="rgba(251,191,36,0.5)" strokeWidth="1" strokeDasharray="2 3"/>
              <path d="M50 12 L50 30 M50 70 L50 88 M12 50 L30 50 M70 50 L88 50" stroke="url(#brand-grad)" strokeWidth="2" strokeLinecap="round"/>
              <circle cx="50" cy="50" r="6" fill="url(#brand-grad)"/>
            </svg>
            <div>
              <div className="wordmark">THE AXIOM SIGNAL CHAIN RACK</div>
              <div className="sub">BIOSPARK STUDIOS · QUILL OS · v0.7.3</div>
            </div>
          </div>
          <div className="chips">
            <div className="chip gold">RACK 19" · 9U</div>
            <div className="chip">EXPORT · BABYLON · UNITY · UNREAL</div>
            <div className="chip live">{(resonance * 432).toFixed(1)}HZ</div>
          </div>
        </div>

        {/* INSTRUMENT BANK SELECTOR */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '6px 12px 14px' }}>
          <div className="instrument-bank">
            {groups.map(g => (
              <button
                key={g}
                className={`ib-btn ${activeGroup === g ? 'active' : ''}`}
                onClick={() => setActiveGroup(g)}
              >
                {g}
              </button>
            ))}
          </div>
          <div style={{ fontFamily: 'var(--font-script)', fontStyle: 'italic', fontSize: 12, color: 'var(--fg-3)' }}>
            Patch outputs into inputs. The rack listens.
          </div>
        </div>

        {/* HIERARCHY BREADCRUMB */}
        <HierCrumb active="mythos" stats={`• ${cables.length} CABLES • ${steps.filter(s=>s.on).length}/16 STEPS • φ${(resonance*432).toFixed(1)}HZ`}/>

        {/* GENESIS MAP — 16 modules grouped by 4 departments */}
        <GenesisMap activeModule="13" onSelect={() => {}}/>

        {/* RACK CHASSIS */}
        <div className="rack">
          {/* LEFT RAIL */}
          <div className="rail">
            <div className="screw"/>
            <div className="screw"/>
            <div className="rail-label">AXIOM / 9U</div>
            <div className="screw"/>
            <div className="screw"/>
          </div>

          {/* MODULES */}
          <div className="rack-body">
            <ConcordanceModule
              resonance={resonance}
              harmonics={harmonics.reduce((a,b)=>a+b,0) / harmonics.length}
              illuminate={illuminate}
              detachment={detachment}
              onIlluminate={setIlluminate}
              onDetachment={setDetachment}
              lissajous={lissajous}
            />

            {inGroup('AXIOM') && (
              <AxiomCarverModule knobs={knobs} setKnob={setKnob}/>
            )}

            {inGroup('PERSONA') && (
              <PersonaForgerModule
                faders={faders} setFader={setFader}
                lfos={lfos} setLfo={setLfo}
              />
            )}

            {inGroup('CHRONO') && (
              <ChronoFlowModule
                steps={steps} toggleStep={toggleStep}
                playhead={playhead} playing={playing}
                onPlay={() => setPlaying(p => !p)}
                jog={jog} setJog={setJog}
                tempo={tempo} setTempo={setTempo}
                swing={swing} setSwing={setSwing}
              />
            )}

            {inGroup('INSTRUMENTS') && (
              <div className="eurorack-row">
                <ResonanceSealerModule harmonics={harmonics}/>
                <MythosCartographerModule activeArc={activeArc} setActiveArc={setActiveArc}/>
                <VoidLatticeModule stress={stress}/>
              </div>
            )}

            {inGroup('INSTRUMENTS') && (
              <NebulaSystemsModule/>
            )}

            <NexusPatchBayModule cables={cables} clearCables={clearCables}/>
          </div>

          {/* RIGHT RAIL */}
          <div className="rail">
            <div className="screw"/>
            <div className="screw"/>
            <div className="rail-label">SIG / CHAIN</div>
            <div className="screw"/>
            <div className="screw"/>
          </div>

          {/* CABLES OVERLAY — spans the rack */}
          <CablesLayer cables={cables} style={t.cableStyle}/>
        </div>

        {/* DOCK */}
        <div className="dock">
          <div className="dock-l">
            <span className="dot"/>
            <span>SYSTEM · COHERENT</span>
            <span>·</span>
            <span>NODES {Object.keys(knobs).length + faders.length + cables.length}</span>
            <span>·</span>
            <span>CABLES {cables.length}</span>
          </div>
          <div className="dock-r">
            <span>SEED 0xA1·X10M</span>
            <span>·</span>
            <span>EPOCH {Math.floor(jog * 12) + 1}/12</span>
            <span>·</span>
            <span style={{ color: 'var(--gold)' }}>"As Above, So Below, So Woven."</span>
          </div>
        </div>

        {/* LORE TOOLTIP */}
        {lore.node}

        {/* TWEAKS PANEL */}
        <TweaksPanel title="Tweaks">
          <TweakSection label="Module identity">
            <TweakRadio
              label="Finish"
              value={t.moduleIdentity}
              onChange={v => setTweak('moduleIdentity', v)}
              options={[
                { value: 'uniform', label: 'Uniform' },
                { value: 'per-module', label: 'Per-module' },
              ]}
            />
          </TweakSection>

          <TweakSection label="Cable style">
            <TweakSelect
              label="Style"
              value={t.cableStyle}
              onChange={v => setTweak('cableStyle', v)}
              options={[
                { value: 'biolum', label: 'Bioluminescent (default)' },
                { value: 'fiber', label: 'Fiber-optic' },
                { value: 'chain', label: 'Chained / dotted' },
              ]}
            />
          </TweakSection>

          <TweakSection label="Background">
            <TweakSelect
              label="Field"
              value={t.background}
              onChange={v => setTweak('background', v)}
              options={[
                { value: 'cosmos', label: 'Cosmos (default)' },
                { value: 'void', label: 'Deep Void' },
                { value: 'nebula', label: 'Nebula wash' },
              ]}
            />
          </TweakSection>

          <TweakSection label="Density">
            <TweakRadio
              label="Pack"
              value={t.density}
              onChange={v => setTweak('density', v)}
              options={[
                { value: 'loose', label: 'Loose' },
                { value: 'standard', label: 'Std' },
                { value: 'packed', label: 'Packed' },
              ]}
            />
          </TweakSection>

          <TweakSection label="Color intensity">
            <TweakRadio
              label="Glow"
              value={t.colorIntensity}
              onChange={v => setTweak('colorIntensity', v)}
              options={[
                { value: 'subtle', label: 'Subtle' },
                { value: 'standard', label: 'Std' },
                { value: 'maximalist', label: 'Max' },
              ]}
            />
          </TweakSection>

          <TweakSection label="Test drive">
            <TweakButton
              label="Randomize all"
              onClick={() => {
                const r = () => Math.random();
                setKnobs({ gravity: r(), causal: r(), entropy: r(), phase: r() });
                setFaders([r(), r(), r(), r(), r()]);
                setLfos({ ambition: r(), survival: r(), curio: r() });
              }}
            />
            <TweakButton
              label={playing ? 'Stop sequencer' : 'Start sequencer'}
              onClick={() => setPlaying(p => !p)}
            />
            <TweakButton
              label="Sever all cables"
              onClick={clearCables}
            />
          </TweakSection>
        </TweaksPanel>
      </div>
    </SignalContext.Provider>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<App/>);
