/* =====================================================
   MODULES — composed from controls + visualizers.
   Each module is a vertical panel with header, body,
   and patch footer. They tile horizontally in a rack.
   ===================================================== */

const { useState: useMState, useMemo: useMMemo, useEffect: useMEffect } = React;

/* ---------- crest glyphs (simple SVG, drawn from the registry) ---------- */
const Crests = {
  atlas: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="9"/>
      <ellipse cx="12" cy="12" rx="9" ry="3.5"/>
      <ellipse cx="12" cy="12" rx="3.5" ry="9"/>
      <path d="M3 12 H21 M12 3 V21"/>
      <path d="M12 1.5 L13 3 L11 3 Z" fill="currentColor" stroke="none"/>
      <circle cx="12" cy="12" r="1" fill="currentColor" stroke="none"/>
    </svg>
  ),
  mythos: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round">
      <circle cx="12" cy="12" r="9"/>
      <ellipse cx="12" cy="12" rx="9" ry="3"/>
      <ellipse cx="12" cy="12" rx="9" ry="3" transform="rotate(60 12 12)"/>
      <ellipse cx="12" cy="12" rx="9" ry="3" transform="rotate(120 12 12)"/>
      <circle cx="12" cy="12" r="1.6" fill="currentColor" stroke="none"/>
    </svg>
  ),
  architect: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round">
      <path d="M3 21 L 12 3 L 21 21 Z"/>
      <path d="M12 3 V21"/>
      <path d="M7.5 12 H 16.5"/>
      <path d="M9.75 7.5 H 14.25"/>
      <path d="M5.25 16.5 H 18.75"/>
      <circle cx="12" cy="3" r="1" fill="currentColor" stroke="none"/>
    </svg>
  ),
  prism: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round">
      <path d="M3 21 L 12 3 L 21 21 Z"/>
      <path d="M3 21 L 12 13 L 21 21"/>
      <path d="M12 3 V13"/>
      <path d="M7 16 L 17 16"/>
    </svg>
  ),
  animus: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round">
      <path d="M12 2 V22 M2 12 H22"/>
      <path d="M5 5 L19 19 M19 5 L5 19"/>
      <circle cx="12" cy="12" r="3.5"/>
      <circle cx="12" cy="12" r="1.4" fill="currentColor" stroke="none"/>
    </svg>
  ),
  loom: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round">
      <polygon points="12,2 21,7 21,17 12,22 3,17 3,7"/>
      <polygon points="12,7 17,9.5 17,14.5 12,17 7,14.5 7,9.5"/>
      <line x1="12" y1="2"  x2="12" y2="7"/>
      <line x1="21" y1="7"  x2="17" y2="9.5"/>
      <line x1="21" y1="17" x2="17" y2="14.5"/>
      <line x1="12" y1="22" x2="12" y2="17"/>
      <line x1="3"  y1="17" x2="7"  y2="14.5"/>
      <line x1="3"  y1="7"  x2="7"  y2="9.5"/>
      <circle cx="12" cy="12" r="1.6" fill="currentColor" stroke="none"/>
    </svg>
  ),
  instinct: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round">
      <path d="M12 2 C 5 2, 5 9, 12 9 S 19 16, 12 16 S 5 22, 12 22"/>
      <circle cx="12" cy="2"  r="1.2" fill="currentColor" stroke="none"/>
      <circle cx="12" cy="22" r="1.2" fill="currentColor" stroke="none"/>
    </svg>
  ),
  order: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round">
      <path d="M3 9 L 7 3 L 12 7 L 17 3 L 21 9 L 19 20 L 5 20 Z"/>
      <path d="M3 9 H 21"/>
      <circle cx="7"  cy="3" r="0.8" fill="currentColor" stroke="none"/>
      <circle cx="12" cy="7" r="0.8" fill="currentColor" stroke="none"/>
      <circle cx="17" cy="3" r="0.8" fill="currentColor" stroke="none"/>
      <path d="M10 13 H 14 M10 16 H 14"/>
    </svg>
  ),
  chronicle: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round">
      <circle cx="12" cy="12" r="10"/>
      <circle cx="12" cy="12" r="7"/>
      <path d="M12 5 V12 L 17 14.5"/>
      <path d="M12 2.5 V4 M12 20 V21.5 M2.5 12 H4 M20 12 H21.5"/>
      <path d="M5 5 L 6 6 M19 19 L 18 18 M5 19 L 6 18 M19 5 L 18 6"/>
      <circle cx="12" cy="12" r="1.2" fill="currentColor" stroke="none"/>
    </svg>
  ),
  quill: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round">
      <polygon points="12,2 21,7 21,17 12,22 3,17 3,7"/>
      <path d="M8 8 L 16 16 M16 8 L 8 16"/>
      <path d="M12 2 V22 M3 7 L 21 17 M21 7 L 3 17" opacity="0.45"/>
      <circle cx="12" cy="12" r="2.2" fill="var(--chassis)"/>
      <circle cx="12" cy="12" r="2.2"/>
    </svg>
  ),
  codex: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round">
      <polygon points="12,2 20,7 17,20 7,20 4,7"/>
      <path d="M8 10 H 16 M8 13 H 16 M8 16 H 13"/>
      <path d="M4 7 L 20 7"/>
      <circle cx="12" cy="2" r="0.9" fill="currentColor" stroke="none"/>
    </svg>
  ),
  composer: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1">
      <circle cx="12" cy="12" r="10"/>
      <circle cx="12" cy="12" r="6.5"/>
      <circle cx="12" cy="12" r="3.2"/>
      <circle cx="12" cy="12" r="1" fill="currentColor" stroke="none"/>
      <path d="M12 2 V5 M12 19 V22 M2 12 H5 M19 12 H22"/>
    </svg>
  ),
  axiom: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round" strokeLinecap="round">
      <path d="M2 7 L 8 7 L 11 12 L 8 17 L 2 17"/>
      <circle cx="14.5" cy="12" r="3.5"/>
      <path d="M18 12 H 22"/>
      <path d="M14.5 8.5 V 15.5"/>
      <circle cx="14.5" cy="12" r="1" fill="currentColor" stroke="none"/>
    </svg>
  ),
  continuum: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round">
      <path d="M2 12 C 4 5, 8 5, 10 12 S 16 19, 18 12 S 22 5, 22 12"/>
      <path d="M2 16 C 4 9, 8 9, 10 16 S 16 23, 18 16 S 22 9, 22 16" opacity="0.4"/>
      <path d="M2 8 C 4 1, 8 1, 10 8 S 16 15, 18 8 S 22 1, 22 8" opacity="0.4"/>
    </svg>
  ),
  forge: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1">
      <circle cx="9"  cy="10" r="5"/>
      <circle cx="15" cy="14" r="3.5"/>
      <circle cx="9"  cy="10" r="2"/>
      <circle cx="15" cy="14" r="1.4"/>
      <circle cx="9"  cy="10" r="0.7" fill="currentColor" stroke="none"/>
      <circle cx="15" cy="14" r="0.5" fill="currentColor" stroke="none"/>
    </svg>
  ),
  nexus: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round">
      <path d="M2 12 C 2 5, 22 5, 22 12 S 2 19, 2 12 Z"/>
      <circle cx="7"  cy="12" r="1.6" fill="currentColor" stroke="none"/>
      <circle cx="12" cy="12" r="1.6" fill="currentColor" stroke="none"/>
      <circle cx="17" cy="12" r="1.6" fill="currentColor" stroke="none"/>
      <path d="M7 12 H 17" opacity="0.4"/>
    </svg>
  ),
};

/* ---------- module shell ---------- */
function Module({ name, num, channel = "cool", crest, width = 200, children, hp }) {
  const computedWidth = hp ? `calc(${hp} * var(--hp))` : width;
  return (
    <div className={`module module-ch-${channel}`}
         style={{
           width: computedWidth,
           "--module-channel": `var(--signal-${channel})`,
           "--module-glow": `var(--glow-${channel})`,
         }}>
      <div className="module-screws">
        <div className="screw tl"/><div className="screw tr"/>
        <div className="screw bl"/><div className="screw br"/>
      </div>
      <div className="module-head">
        {crest && <div className="module-crest" style={{color: `var(--signal-${channel})`}}>{Crests[crest] || crest}</div>}
        <div className="module-name">{name}</div>
        {num && <div className="module-num">{num}</div>}
      </div>
      <div className="module-body">{children}</div>
    </div>
  );
}

/* =====================================================
   PRESET MODULES — used in the Tornado patch
   ===================================================== */

/* --- 1. ATLAS · Terrain Source ---------------------- */
function ModuleAtlas() {
  return (
    <Module name="ATLAS" num="01" channel="cool" crest="atlas" hp="14">
      <div className="module-section">
        <div className="module-section-label">Heightfield</div>
        <ParticleField width={188} height={70} channel="cool" label="TERRAIN" count={50}/>
      </div>
      <div className="module-section">
        <div className="module-section-label">Sample</div>
        <div className="module-row" style={{justifyContent:"space-around"}}>
          <Knob label="LAT"  channel="cool" variant="arc"     size={48} ticks={11} defaultValue={0.42}/>
          <Knob label="LON"  channel="cool" variant="dotted"  size={48} ticks={11} defaultValue={0.61}/>
          <Knob label="ALT"  channel="cool" variant="ringed"  size={48} defaultValue={0.28}/>
        </div>
      </div>
      <div className="module-patch">
        <div className="patch-group">
          <div className="patch-group-label">OUT</div>
          <div className="patch-group-jacks">
            <Jack label="X" channel="cool" active />
            <Jack label="Y" channel="cool" active />
            <Jack label="H" channel="cool" />
            <Jack label="∇" channel="cool" />
          </div>
        </div>
      </div>
    </Module>
  );
}

/* --- 2. CONTINUUM · Simulation Engine --------------- */
function ModuleContinuum() {
  return (
    <Module name="CONTINUUM" num="14" channel="life" crest="continuum" hp="16">
      <div className="module-section">
        <div className="module-section-label">Vortex Field</div>
        <Polar size={148} channel="life" label="ENSTROPHY"/>
      </div>
      <div className="module-section">
        <div className="module-section-label">Sim · 60 fps</div>
        <div className="module-row" style={{justifyContent:"space-between"}}>
          <Knob label="VISC"   channel="life" variant="arc"    size={42} defaultValue={0.18}/>
          <Knob label="VORT"   channel="life" variant="forge"  size={42} defaultValue={0.74}/>
          <Knob label="STEPS"  channel="life" variant="dotted" size={42} ticks={9} defaultValue={0.5}/>
          <Knob label="RES"    channel="life" variant="pip"    size={42} defaultValue={0.62}/>
        </div>
      </div>
      <div className="module-row" style={{gap:6}}>
        <GateBtn label="RUN"  lit channel="life"/>
        <GateBtn label="STEP"      channel="amber"/>
        <GateBtn label="RESET"     channel="hot"/>
      </div>
      <div className="module-patch">
        <div className="patch-group">
          <div className="patch-group-label">IN</div>
          <div className="patch-group-jacks">
            <Jack label="X" channel="cool" active/>
            <Jack label="Y" channel="cool" active/>
            <Jack label="F" channel="warm"/>
          </div>
        </div>
        <div className="patch-group">
          <div className="patch-group-label">OUT</div>
          <div className="patch-group-jacks">
            <Jack label="V" channel="life" active/>
            <Jack label="ω" channel="life" active/>
            <Jack label="P" channel="life"/>
          </div>
        </div>
      </div>
    </Module>
  );
}

/* --- 3. INSTINCT · Behavior Modulator --------------- */
function ModuleInstinct() {
  return (
    <Module name="INSTINCT" num="07" channel="myth" crest="instinct" hp="13">
      <div className="module-section">
        <div className="module-section-label">Envelope · ATK / DCY / SUS / RLS</div>
        <CurveEditor width={172} height={70} channel="myth" label="ADSR"/>
      </div>
      <div className="module-section">
        <div className="module-section-label">LFO Bank</div>
        <div className="module-row" style={{justifyContent:"space-around"}}>
          <Knob label="RATE"  channel="myth" variant="arc"    size={40} defaultValue={0.34}/>
          <Knob label="DEPTH" channel="myth" variant="ringed" size={40} defaultValue={0.66}/>
          <Knob label="SHAPE" channel="myth" variant="dotted" size={40} ticks={8} defaultValue={0.5}/>
        </div>
      </div>
      <div className="module-patch">
        <div className="patch-group">
          <div className="patch-group-label">CV</div>
          <div className="patch-group-jacks">
            <Jack label="1" channel="myth" active/>
            <Jack label="2" channel="myth"/>
            <Jack label="3" channel="myth"/>
            <Jack label="4" channel="myth"/>
          </div>
        </div>
      </div>
    </Module>
  );
}

/* --- 4. CHRONICLE · Sequencer ----------------------- */
function ModuleChronicle({ step }) {
  return (
    <Module name="CHRONICLE" num="09" channel="amber" crest="chronicle" hp="20">
      <div className="module-section">
        <div className="module-section-label">Steps · 16</div>
        <StepRow steps={16} current={step} channel="amber"/>
        <StepRow steps={16} current={step} channel="warm"/>
      </div>
      <div className="module-row" style={{justifyContent:"space-between"}}>
        <Readout label="BPM" value="124.0" channel="amber" width={76}/>
        <Readout label="STEP" value={`${(step+1).toString().padStart(2,"0")}/16`} channel="amber" width={76}/>
        <Readout label="DIV" value="1/16" channel="amber" width={76}/>
        <Knob label="SWING" channel="amber" variant="arc" size={42} defaultValue={0.12} bipolar/>
      </div>
      <div className="module-patch">
        <div className="patch-group">
          <div className="patch-group-label">CLK</div>
          <div className="patch-group-jacks">
            <Jack label="◍" channel="amber" active={step%4===0}/>
            <Jack label="↺" channel="amber"/>
          </div>
        </div>
        <div className="patch-group">
          <div className="patch-group-label">GATES · A B</div>
          <div className="patch-group-jacks">
            <Jack label="A" channel="life" active/>
            <Jack label="B" channel="life"/>
          </div>
        </div>
        <div className="patch-group">
          <div className="patch-group-label">CV</div>
          <div className="patch-group-jacks">
            <Jack label="1" channel="cool" active/>
            <Jack label="2" channel="myth"/>
          </div>
        </div>
      </div>
    </Module>
  );
}

/* --- 5. COMPOSER · Audio Voicing -------------------- */
function ModuleComposer() {
  return (
    <Module name="COMPOSER" num="12" channel="warm" crest="composer" hp="14">
      <div className="module-section">
        <div className="module-section-label">Spectrum · 0–20kHz</div>
        <Spectrum width={188} height={70} bands={28} channel="warm"/>
      </div>
      <div className="module-row" style={{justifyContent:"space-between"}}>
        <Fader label="GAIN" channel="warm" height={52}/>
        <Knob label="PITCH" channel="warm" variant="arc" size={42} bipolar defaultValue={0.5}/>
        <Knob label="DRIVE" channel="forge" variant="forge" size={42} defaultValue={0.6}/>
        <Knob label="FORMANT" channel="warm" variant="dotted" size={42} ticks={7} defaultValue={0.4}/>
      </div>
      <div className="module-row">
        <VU width={92} channel="life" label="L"/>
        <VU width={92} channel="life" label="R"/>
      </div>
      <div className="module-patch">
        <div className="patch-group">
          <div className="patch-group-label">IN</div>
          <div className="patch-group-jacks">
            <Jack label="V" channel="life" active/>
            <Jack label="ω" channel="life" active/>
          </div>
        </div>
        <div className="patch-group">
          <div className="patch-group-label">AUDIO</div>
          <div className="patch-group-jacks">
            <Jack label="L" channel="warm" active/>
            <Jack label="R" channel="warm" active/>
          </div>
        </div>
      </div>
    </Module>
  );
}

/* --- 6. QUILL · Story / Narrative ------------------- */
function ModuleQuill() {
  return (
    <Module name="QUILL" num="10" channel="myth" crest="quill" hp="12">
      <div className="module-section">
        <div className="module-section-label">Beat Map</div>
        <PianoRoll width={156} height={62} channel="myth"/>
      </div>
      <div className="module-section">
        <div className="module-section-label">Arc</div>
        <div className="module-row" style={{justifyContent:"space-around"}}>
          <Knob label="ACT"     channel="myth" variant="dotted" ticks={5} size={44} defaultValue={0.6}/>
          <Knob label="TENSION" channel="rose" variant="arc"    size={44} defaultValue={0.78}/>
        </div>
      </div>
      <div className="module-section">
        <XYPad size={140} channel="myth" label="VALENCE × AROUSAL"/>
      </div>
      <div className="module-patch">
        <div className="patch-group">
          <div className="patch-group-label">NARR</div>
          <div className="patch-group-jacks">
            <Jack label="↗" channel="myth" active/>
            <Jack label="✦" channel="myth"/>
            <Jack label="↻" channel="myth"/>
          </div>
        </div>
      </div>
    </Module>
  );
}

/* --- 7. FORGE · Audio Output / Master --------------- */
function ModuleForge() {
  return (
    <Module name="FORGE" num="15" channel="forge" crest="forge" hp="10">
      <div className="module-section">
        <div className="module-section-label">Master</div>
        <div className="center" style={{padding:"6px 0"}}>
          <Knob label="MAIN" channel="forge" variant="forge" size={68} ticks={21} defaultValue={0.78}/>
        </div>
      </div>
      <div className="module-section">
        <div className="module-section-label">Limiter</div>
        <VU width={120} channel="life" label="L"/>
        <VU width={120} channel="life" label="R"/>
        <VU width={120} channel="warm" label="◐"/>
      </div>
      <div className="module-section">
        <div className="module-row" style={{justifyContent:"space-between"}}>
          <Switch positions={3} labels={["MUTE","CUE","ON"]} channel="forge"/>
          <Pad label="ARM" channel="hot" lit size={32}/>
          <Pad label="REC" channel="hot" size={32}/>
        </div>
      </div>
      <div className="module-patch">
        <div className="patch-group">
          <div className="patch-group-label">MAIN</div>
          <div className="patch-group-jacks">
            <Jack label="L" channel="forge" active/>
            <Jack label="R" channel="forge" active/>
          </div>
        </div>
      </div>
    </Module>
  );
}

/* expose */
Object.assign(window, {
  Module, Crests,
  ModuleAtlas, ModuleContinuum, ModuleInstinct, ModuleChronicle,
  ModuleComposer, ModuleQuill, ModuleForge,
});
