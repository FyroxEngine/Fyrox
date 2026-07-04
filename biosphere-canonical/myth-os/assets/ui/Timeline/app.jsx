/* =====================================================
   APP — the showcase page assembling everything
   ===================================================== */

const { useState: useAState, useEffect: useAEffect } = React;

/* ---- 16-DEPT REGISTRY ---- */
const REGISTRY = [
  { num:"01", dept:"I",   law:"Space",        crest:"atlas",     name:"ATLAS",     color:"--ch01-atlas",     ch:"cool",  emblem:"Globe + Compass" },
  { num:"02", dept:"I",   law:"World",        crest:"mythos",    name:"MYTHOS",    color:"--ch02-mythos",    ch:"myth",  emblem:"Armillary Sphere" },
  { num:"03", dept:"I",   law:"Structure",    crest:"architect", name:"ARCHITECT", color:"--ch03-architect", ch:"cool",  emblem:"Compass & Square" },
  { num:"04", dept:"I",   law:"Light",        crest:"prism",     name:"PRISM",     color:"--ch04-prism",     ch:"bone",  emblem:"Prism Triangle" },
  { num:"05", dept:"II",  law:"Character",    crest:"animus",    name:"ANIMUS",    color:"--ch05-animus",    ch:"amber", emblem:"Asterisk" },
  { num:"06", dept:"II",  law:"Connection",   crest:"loom",      name:"LOOM",      color:"--ch06-loom",      ch:"rose",  emblem:"Hex Web" },
  { num:"07", dept:"II",  law:"Reason",       crest:"instinct",  name:"INSTINCT",  color:"--ch07-instinct",  ch:"myth",  emblem:"Spiral Synapse" },
  { num:"08", dept:"II",  law:"Authority",    crest:"order",     name:"ORDER",     color:"--ch08-order",     ch:"amber", emblem:"Crown-Seal" },
  { num:"09", dept:"III", law:"Time",         crest:"chronicle", name:"CHRONICLE", color:"--ch09-chronicle", ch:"amber", emblem:"Clockwork Spiral" },
  { num:"10", dept:"III", law:"Narrative",    crest:"quill",     name:"QUILL",     color:"--ch10-quill",     ch:"myth",  emblem:"Hexfeather" },
  { num:"11", dept:"III", law:"Knowledge",    crest:"codex",     name:"CODEX",     color:"--ch11-codex",     ch:"life",  emblem:"Pentagon Book" },
  { num:"12", dept:"III", law:"Audio",        crest:"composer",  name:"COMPOSER",  color:"--ch12-composer",  ch:"warm",  emblem:"Vinyl" },
  { num:"13", dept:"IV",  law:"Rules",        crest:"axiom",     name:"AXIOM",     color:"--ch13-axiom",     ch:"cool",  emblem:"Logic Gate" },
  { num:"14", dept:"IV",  law:"Operation",    crest:"continuum", name:"CONTINUUM", color:"--ch14-continuum", ch:"life",  emblem:"Recursive Loop" },
  { num:"15", dept:"IV",  law:"Creation",     crest:"forge",     name:"FORGE",     color:"--ch15-forge",     ch:"forge", emblem:"Counter Gears" },
  { num:"16", dept:"IV",  law:"Bridge",       crest:"nexus",     name:"NEXUS",     color:"--ch16-nexus",     ch:"bone",  emblem:"Infinity Node" },
];

/* ---- HERO IDENTITY MARK ---- */
function IdentityMark() {
  return (
    <svg width="160" height="160" viewBox="0 0 160 160">
      <defs>
        <radialGradient id="im-bg" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="rgba(34,209,255,0.3)"/>
          <stop offset="100%" stopColor="transparent"/>
        </radialGradient>
        <linearGradient id="im-ring" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="var(--signal-cool)"/>
          <stop offset="50%" stopColor="var(--signal-myth)"/>
          <stop offset="100%" stopColor="var(--signal-rose)"/>
        </linearGradient>
      </defs>
      <circle cx="80" cy="80" r="78" fill="url(#im-bg)"/>
      {/* outer dotted ring */}
      <circle cx="80" cy="80" r="72" fill="none" stroke="rgba(255,255,255,0.15)"
              strokeWidth="1" strokeDasharray="2 4"/>
      {/* mid neon ring */}
      <circle cx="80" cy="80" r="58" fill="none" stroke="url(#im-ring)" strokeWidth="2"
              style={{filter: "drop-shadow(0 0 6px var(--signal-cool))"}}/>
      {/* tick marks every 30deg */}
      {Array.from({length:24}).map((_,i)=>{
        const a = (i/24)*Math.PI*2;
        const r1 = 52, r2 = i%2===0 ? 46 : 49;
        return <line key={i}
          x1={80+Math.cos(a)*r1} y1={80+Math.sin(a)*r1}
          x2={80+Math.cos(a)*r2} y2={80+Math.sin(a)*r2}
          stroke="rgba(255,255,255,0.4)" strokeWidth="1"/>
      })}
      {/* central glyph: triangular spark inside hex */}
      <polygon points="80,30 124,55 124,105 80,130 36,105 36,55"
               fill="rgba(0,0,0,0.6)" stroke="var(--signal-cool)" strokeWidth="1.5"
               style={{filter: "drop-shadow(0 0 4px var(--signal-cool))"}}/>
      <path d="M 80 50 L 100 95 L 60 95 Z" fill="none" stroke="var(--signal-myth)" strokeWidth="1.5"
            style={{filter: "drop-shadow(0 0 4px var(--signal-myth))"}}/>
      <circle cx="80" cy="80" r="5" fill="var(--signal-rose)"
              style={{filter: "drop-shadow(0 0 8px var(--signal-rose))"}}/>
      {/* corner crosshairs */}
      {[[0,0],[160,0],[0,160],[160,160]].map(([x,y],i)=>(
        <g key={i} stroke="var(--signal-cool)" strokeWidth="1" opacity="0.5">
          <line x1={x} y1={y} x2={x + (x===0?12:-12)} y2={y}/>
          <line x1={x} y1={y} x2={x} y2={y + (y===0?12:-12)}/>
        </g>
      ))}
    </svg>
  );
}

/* ---- SECTION HEADER ---- */
function SectionHead({ num, title, blurb }) {
  return (
    <div className="section-head">
      <div className="section-num">{num}</div>
      <h2 className="section-title">{title}</h2>
      {blurb && <div className="section-blurb">{blurb}</div>}
    </div>
  );
}

/* ---- ICON NEST: 4 concentric boxes (100/75/50/25%), each with a copy of
       the same glyph filling its frame. Smallest sits where the original
       24-px icon used to live. ---- */
function IconNest({ title, colorVar, children }) {
  const tone = `var(${colorVar})`;
  return (
    <div className="icon-cell" title={title}
         style={{color: tone, filter: `drop-shadow(0 0 4px ${tone})`}}>
      <div className="icon-nest">
        <div className="nest nest-1">{children}</div>
        <div className="nest nest-2">{children}</div>
        <div className="nest nest-3">{children}</div>
        <div className="nest nest-4">{children}</div>
      </div>
    </div>
  );
}

/* ---- MAIN APP ---- */
function App() {
  return (
    <div className="page">
      {/* HERO --------------------------------------- */}
      <section className="hero">
        <div className="identity">
          <div className="identity-mark">
            <IdentityMark/>
            <div className="hero-mark">TIMELINE · BIOSPARK STUDIOS</div>
          </div>
          <div style={{flex:2, display:"flex", flexDirection:"column", justifyContent:"center", gap:24}}>
            <div className="hero-mark">DESIGN SYSTEM · v0.7 · PROCEDURAL RACK</div>
            <h1 className="hero-title">The Performable<br/>Universe.</h1>
            <p className="hero-sub">
              A control system for the narrative engine. Tornadoes, cities, characters, dying stars —
              every simulated thing exposes knobs, gates, jacks. Patch them together and play the world
              like an instrument. This document defines the chassis, channels, controls, visualizers
              and module grammar for an infinite VCV-style rack.
            </p>
            <div className="hero-meta">
              <div>SYSTEM<strong>16 channels · 7 chassis</strong></div>
              <div>SAMPLE RATE<strong>48kHz · 60Hz CV</strong></div>
              <div>NODE TYPES<strong>148</strong></div>
              <div>STATUS<strong style={{color:"var(--signal-life)"}}>● LIVE</strong></div>
            </div>
          </div>
        </div>
      </section>

      {/* 01 — REGISTRY ------------------------------ */}
      <section className="section">
        <SectionHead num="01" title="Department Registry"
          blurb="Sixteen channels of authority. Each department contributes a hue, a crest, and an operator family. Knobs and jacks inherit channel color from the module they live on."/>
        <div className="registry">
          {REGISTRY.map(r => (
            <div key={r.num} className="reg-card" style={{ "--reg-color": `var(${r.color})` }}>
              <div className="reg-crest">{Crests[r.crest]}</div>
              <div className="reg-info">
                <div className="reg-num">{r.num} · LAW {r.dept} · {r.law.toUpperCase()}</div>
                <div className="reg-name">{r.name}</div>
                <div className="reg-meta">{r.emblem}</div>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* 02 — SIGNAL COLORS ------------------------- */}
      <section className="section">
        <SectionHead num="02" title="Signal Colors"
          blurb="Functional palette overlay. Used independently of department channels for control state, gates, alerts, and signal classification across patches."/>
        <div className="tok-grid">
          {[
            {name:"COOL",  v:"--signal-cool",  use:"Primary CV / data"},
            {name:"LIFE",  v:"--signal-life",  use:"Armed · alive · running"},
            {name:"MYTH",  v:"--signal-myth",  use:"Narrative · LFO · arc"},
            {name:"WARM",  v:"--signal-warm",  use:"Audio · heat · drive"},
            {name:"AMBER", v:"--signal-amber", use:"Caution · clock · time"},
            {name:"ROSE",  v:"--signal-rose",  use:"Resonance · valence"},
            {name:"HOT",   v:"--signal-hot",   use:"Clip · alert · armed-rec"},
            {name:"FORGE", v:"--signal-forge", use:"Master out · creation"},
          ].map(c => (
            <div key={c.name} className="tok-swatch">
              <div className="tok-swatch-disc" style={{
                background: `radial-gradient(circle at 35% 30%, var(${c.v}), color-mix(in srgb, var(${c.v}) 30%, #050810))`,
                boxShadow: `0 0 16px var(${c.v}), inset 0 0 8px rgba(255,255,255,0.2)`,
              }}/>
              <div>
                <div className="tok-swatch-name">{c.name}</div>
                <div className="tok-swatch-num">{c.v}</div>
              </div>
              <div style={{fontSize:10, color:"var(--ink-mid)"}}>{c.use}</div>
            </div>
          ))}
        </div>
      </section>

      {/* 03 — TYPOGRAPHY ---------------------------- */}
      <section className="section">
        <SectionHead num="03" title="Typography"
          blurb="Four roles. Display for module marks, mono for values, engrave for chassis labels, text for body. Letterspacing widens with smaller sizes — etched-into-metal feel."/>
        <div className="type-spec">
          <div className="type-row">
            <div className="type-row-meta"><strong>DISPLAY</strong>Orbitron · 88/56/40/28<br/>letter-spacing 0.04em</div>
            <div className="type-row-sample" style={{fontFamily:"var(--font-display)", fontSize:56, letterSpacing:"0.04em", textTransform:"uppercase", fontWeight:700}}>The Performable Universe</div>
          </div>
          <div className="type-row">
            <div className="type-row-meta"><strong>ENGRAVE</strong>Eurostile / Orbitron<br/>9–12px · 0.18em tracking</div>
            <div className="type-row-sample engrave" style={{fontSize:14}}>CHRONICLE · 09 · TIME · CLOCK · STEP · GATE A · GATE B</div>
          </div>
          <div className="type-row">
            <div className="type-row-meta"><strong>READOUT</strong>JetBrains Mono · 11px<br/>tabular nums · glow</div>
            <div className="type-row-sample readout" style={{fontSize:18}}>124.0 BPM · 048/120 · 0.75v · ω 2.34 · 17.3kHz</div>
          </div>
          <div className="type-row">
            <div className="type-row-meta"><strong>BODY</strong>Inter · 13/14/16<br/>1.45 line-height</div>
            <div className="type-row-sample">Patch a tornado into a granular voice. The vortex's enstrophy modulates pitch; the pressure differential becomes drive; the path it traces becomes a beat map. The world plays itself and you play the world.</div>
          </div>
        </div>
      </section>

      {/* 04 — MATERIALS ----------------------------- */}
      <section className="section">
        <SectionHead num="04" title="Materials"
          blurb="Five surfaces compose every module. Cavity recedes, panel rises, screen glows, knurl bites, ink etches. They never appear in isolation — always layered."/>
        <div className="mat-grid">
          <div className="mat-cell">
            <div className="mat-cell-vis mat-cavity"/>
            <div className="mat-cell-name">Cavity</div>
            <div className="mat-cell-meta">recessed substrate · sh-deep</div>
          </div>
          <div className="mat-cell">
            <div className="mat-cell-vis mat-panel"/>
            <div className="mat-cell-name">Panel</div>
            <div className="mat-cell-meta">brushed face · sh-raise</div>
          </div>
          <div className="mat-cell">
            <div className="mat-cell-vis mat-screen" style={{padding:8}}>
              <Spectrum width={200} height={84} bands={20} channel="cool" label=""/>
            </div>
            <div className="mat-cell-name">Screen</div>
            <div className="mat-cell-meta">glass display · sh-screen</div>
          </div>
          <div className="mat-cell">
            <div className="mat-cell-vis" style={{
              background: "repeating-conic-gradient(from 0deg, #2a2f3a 0deg, #1a1d24 1.5deg, #2a2f3a 3deg)",
              borderRadius: "50%", aspectRatio: "1", height: "auto", margin: "0 auto",
              boxShadow: "inset 0 1px 0 rgba(255,255,255,0.1), 0 4px 8px rgba(0,0,0,0.6)",
              width: 100,
            }}/>
            <div className="mat-cell-name">Knurl</div>
            <div className="mat-cell-meta">milled metal · forge bezels</div>
          </div>
        </div>
      </section>

      {/* 05 — CONTROLS ------------------------------ */}
      <section className="section">
        <SectionHead num="05" title="Controls · Atomic"
          blurb="Drag any knob vertically. Click jacks to patch. Faders track pointer. Every control accepts a channel prop and inherits its parent module's hue."/>

        <h3 style={{font:"600 13px var(--font-display)", letterSpacing:"0.18em", color:"var(--ink-mid)", margin:"32px 0 12px"}}>5 KNOB VARIANTS</h3>
        <div className="demo-grid">
          <div className="demo-cell"><Knob label="CLASSIC"  variant="classic" channel="cool"  size={64} ticks={11}/><div className="demo-cell-name">classic · ticks</div></div>
          <div className="demo-cell"><Knob label="ARC"      variant="arc"     channel="myth"  size={64}/>          <div className="demo-cell-name">arc · neon ring</div></div>
          <div className="demo-cell"><Knob label="DOTTED"   variant="dotted"  channel="warm"  size={64} ticks={9}/> <div className="demo-cell-name">dotted halo</div></div>
          <div className="demo-cell"><Knob label="RINGED"   variant="ringed"  channel="life"  size={64}/>          <div className="demo-cell-name">ringed · pip</div></div>
          <div className="demo-cell"><Knob label="FORGE"    variant="forge"   channel="forge" size={64}/>          <div className="demo-cell-name">forge · knurled</div></div>
          <div className="demo-cell"><Knob label="BIPOLAR"  variant="arc"     channel="rose"  size={64} bipolar defaultValue={0.5}/><div className="demo-cell-name">bipolar · ±1</div></div>
        </div>

        <h3 style={{font:"600 13px var(--font-display)", letterSpacing:"0.18em", color:"var(--ink-mid)", margin:"32px 0 12px"}}>FADERS · JACKS · SWITCHES · PADS</h3>
        <div className="demo-grid">
          <div className="demo-cell">
            <div style={{display:"flex", gap:12}}>
              <Fader label="L"   channel="life"  height={120}/>
              <Fader label="R"   channel="life"  height={120}/>
              <Fader label="DRY" channel="warm"  height={120}/>
              <Fader label="WET" channel="myth"  height={120}/>
            </div>
            <div className="demo-cell-name">faders · 4ch</div>
          </div>
          <div className="demo-cell">
            <div style={{display:"flex", gap:6, alignItems:"flex-end", flexWrap:"wrap", justifyContent:"center"}}>
              <Jack label="OUT" channel="cool" active/>
              <Jack label="CV"  channel="myth"/>
              <Jack label="GT"  channel="life" active/>
              <Jack label="CK"  channel="amber"/>
              <Jack label="L"   channel="warm" active/>
              <Jack label="R"   channel="warm" active/>
              <Jack label="↗"   channel="rose"/>
              <Jack label="✦"   channel="myth" active/>
            </div>
            <div className="demo-cell-name">jacks · patch points</div>
          </div>
          <div className="demo-cell">
            <div style={{display:"flex", gap:12, alignItems:"center"}}>
              <Switch positions={2} channel="cool"/>
              <Switch positions={3} channel="myth"/>
              <Switch positions={3} channel="forge"/>
            </div>
            <div className="demo-cell-name">switches · 2/3-pos</div>
          </div>
          <div className="demo-cell">
            <div style={{display:"grid", gridTemplateColumns:"repeat(4,1fr)", gap:6}}>
              <Pad label="A" channel="rose" size={28}/>
              <Pad label="B" channel="rose" lit size={28}/>
              <Pad label="C" channel="warm" size={28}/>
              <Pad label="D" channel="warm" lit size={28}/>
              <Pad label="E" channel="life" lit size={28}/>
              <Pad label="F" channel="life" size={28}/>
              <Pad label="G" channel="myth" size={28}/>
              <Pad label="H" channel="myth" lit size={28}/>
            </div>
            <div className="demo-cell-name">trigger pads · 4×2</div>
          </div>
          <div className="demo-cell">
            <div style={{display:"flex", flexDirection:"column", gap:6, width:"100%", alignItems:"center"}}>
              <GateBtn label="ARMED" channel="hot" lit/>
              <GateBtn label="LIVE"  channel="life" lit/>
              <GateBtn label="LOOP"  channel="myth"/>
              <GateBtn label="HOLD"  channel="amber"/>
            </div>
            <div className="demo-cell-name">gate buttons</div>
          </div>
          <div className="demo-cell">
            <div style={{display:"flex", flexDirection:"column", gap:4, alignItems:"center"}}>
              <Readout label="BPM"   value="124.0" channel="amber" width={92}/>
              <Readout label="GAIN"  value="-3.2dB" channel="life" width={92}/>
              <Readout label="FREQ"  value="17.3k" channel="cool" width={92}/>
              <Readout label="STEP"  value="07/16" channel="myth" width={92}/>
            </div>
            <div className="demo-cell-name">digital readouts</div>
          </div>
          <div className="demo-cell">
            <div style={{display:"flex", gap:6, flexWrap:"wrap", justifyContent:"center", alignItems:"center"}}>
              <LED on channel="life"/><LED on channel="cool"/><LED channel="amber"/>
              <LED on channel="rose"/><LED on channel="myth"/><LED channel="life"/>
              <LED on channel="hot" size={8}/><LED on channel="warm" size={8}/><LED channel="cool" size={8}/>
            </div>
            <div className="demo-cell-name">LEDs · indicators</div>
          </div>
          <div className="demo-cell">
            <XYPad size={120} channel="rose" label="VAL × AROUSAL"/>
            <div className="demo-cell-name">XY pad · 2D mod</div>
          </div>
        </div>

        <h3 style={{font:"600 13px var(--font-display)", letterSpacing:"0.18em", color:"var(--ink-mid)", margin:"32px 0 12px"}}>STEP SEQUENCER ROWS</h3>
        <div style={{display:"flex", flexDirection:"column", gap:8, padding:"16px", background:"var(--chassis)", borderRadius:6, boxShadow:"inset 0 1px 0 rgba(255,255,255,0.04)"}}>
          <StepRow steps={16} channel="cool"  current={6}/>
          <StepRow steps={16} channel="life"  current={6}/>
          <StepRow steps={16} channel="myth"  current={6}/>
          <StepRow steps={16} channel="warm"  current={6}/>
        </div>
      </section>

      {/* 06 — VISUALIZERS --------------------------- */}
      <section className="section">
        <SectionHead num="06" title="Visualizers"
          blurb="Glass screens carrying live signal. All canvases redraw at 60fps. Peak holds, scrolling spectrograms, particle flows, Lissajous traces — pick any to wear a department's color."/>
        <div className="viz-grid">
          <Scope         width={280} height={100} channel="cool"  freq={2.4} label="OSCILLOSCOPE"/>
          <Spectrum      width={280} height={100} bands={32}      channel="life"  label="SPECTRUM · 32 BANDS"/>
          <Spectrogram   width={280} height={100}                  label="SPECTROGRAM"/>
          <PhaseScope    size={140}                                channel="rose" label="PHASE / XY"/>
          <Polar         size={140}                                channel="myth" label="RADAR"/>
          <ParticleField width={280} height={140} channel="cool"   label="FLOW FIELD"/>
          <CurveEditor   width={280} height={120} channel="myth"   label="ENVELOPE · DRAG POINTS"/>
          <PianoRoll     width={280} height={100} channel="warm"   label="PIANO ROLL"/>
          <Waveform      width={280} height={64}  channel="rose"   label="WAVEFORM"/>
          <NodeMinimap   width={280} height={120}                  label="NODE MINIMAP"/>
          <div style={{padding:8}}>
            <div className="vu" style={{flexDirection:"column", gap:6, alignItems:"stretch", width:280}}>
              <VU width={280} height={14} channel="life"  label="L"/>
              <VU width={280} height={14} channel="life"  label="R"/>
              <VU width={280} height={14} channel="amber" label="◐"/>
              <VU width={280} height={14} channel="hot"   label="◑"/>
            </div>
          </div>
        </div>
      </section>

      {/* 07 — MODULE GRAMMAR ------------------------ */}
      <section className="section">
        <SectionHead num="07" title="Module Grammar"
          blurb="A module is HEAD (crest, name, num) → BODY (sections of controls + screens) → PATCH (jacks). Width is measured in HP — 1HP = 16px. Channel color flows from header stripe to crest glow to jack rims."/>
        <div style={{display:"flex", gap:16, flexWrap:"wrap", padding:24, background:"var(--abyss)", borderRadius:8, boxShadow:"inset 0 0 0 1px rgba(0,0,0,0.6), inset 0 2px 8px rgba(0,0,0,0.7)"}}>
          <ModuleAtlas/>
          <ModuleInstinct/>
          <ModuleQuill/>
          <ModuleForge/>
        </div>
      </section>

      {/* 08 — PATCH: TORNADO ------------------------ */}
      <section className="section">
        <SectionHead num="08" title="Patch · TORNADO"
          blurb="The world's first vortex turned into a granulator. Atlas samples a heightfield → Continuum solves the storm → Instinct shapes its envelope → Chronicle drives the beat → Composer voices it → Quill wraps it in narrative arc → Forge sends it to the listener. Cables glow with their channel's color and pulse with signal."/>
        <TornadoRack/>
        <div style={{display:"flex", gap:24, marginTop:16, fontSize:11, color:"var(--ink-mid)", fontFamily:"var(--font-mono)", letterSpacing:"0.08em"}}>
          <div><span style={{color:"var(--signal-cool)", textShadow:"0 0 4px var(--signal-cool)"}}>━━━</span> CV · data</div>
          <div><span style={{color:"var(--signal-life)", textShadow:"0 0 4px var(--signal-life)"}}>━━━</span> velocity · simulation</div>
          <div><span style={{color:"var(--signal-myth)", textShadow:"0 0 4px var(--signal-myth)"}}>━━━</span> modulation · narrative</div>
          <div><span style={{color:"var(--signal-warm)", textShadow:"0 0 4px var(--signal-warm)"}}>━━━</span> audio</div>
          <div><span style={{color:"var(--signal-amber)", textShadow:"0 0 4px var(--signal-amber)"}}>━━━</span> clock</div>
        </div>
      </section>

      {/* 09 — NODE GRAPH ---------------------------- */}
      <section className="section">
        <SectionHead num="09" title="Node Graph · Houdini View"
          blurb="The same patch, decomposed. Each module unrolls into a sub-graph of operators with display/render/lock flags. Wires inherit their source's channel color. Right-click any node to drop into its rack."/>
        <NodeGraph/>
      </section>

      {/* 10 — TIMELINE ------------------------------ */}
      <section className="section">
        <SectionHead num="10" title="Timeline · Maya View"
          blurb="Performances are time. Every knob is automatable; every gate is a keyframe. Tracks color-coded by source department. The amber playhead is the only thing moving — tempo is dictated by Chronicle."/>
        <Timeline/>
      </section>

      {/* 11 — ICONOGRAPHY --------------------------- */}
      <section className="section">
        <SectionHead num="11" title="Iconography"
          blurb="Crest glyphs, signal types, transport, and operator marks. Drawn on a 24-grid with 1.1px stroke. Each cell shows the same glyph at four nested scales — 100 / 75 / 50 / 25 % — so we can sanity-check every mark down to a 6px favicon. Glyphs scale with the box; they always nearly fill it."/>
        <div className="icons">
          {REGISTRY.map(r => (
            <IconNest key={r.num} title={r.name} colorVar={r.color}>
              {Crests[r.crest]}
            </IconNest>
          ))}
          {/* signal type / transport / utility */}
          {[
            {name:"play",  ch:"life", svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round">
                <circle cx="12" cy="12" r="10"/>
                <path d="M9.5 7 L 17 12 L 9.5 17 Z" fill="currentColor" stroke="none"/>
              </svg>
            )},
            {name:"stop",  ch:"hot",  svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round">
                <circle cx="12" cy="12" r="10"/>
                <rect x="7" y="7" width="10" height="10" rx="1" fill="currentColor" stroke="none"/>
              </svg>
            )},
            {name:"rec",   ch:"hot",  svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1">
                <circle cx="12" cy="12" r="10"/>
                <circle cx="12" cy="12" r="6.5"/>
                <circle cx="12" cy="12" r="4" fill="currentColor" stroke="none"/>
              </svg>
            )},
            {name:"pause", ch:"amber",svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round">
                <circle cx="12" cy="12" r="10"/>
                <rect x="7.5" y="6.5" width="3" height="11" rx="0.6" fill="currentColor" stroke="none"/>
                <rect x="13.5" y="6.5" width="3" height="11" rx="0.6" fill="currentColor" stroke="none"/>
              </svg>
            )},
            {name:"loop",  ch:"myth", svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M5 11 a 7 7 0 1 1 0 2"/>
                <path d="M3 7 L 5 11 L 9 9"/>
                <path d="M5 13 a 7 7 0 0 0 14 0" opacity="0.4"/>
              </svg>
            )},
            {name:"sync",  ch:"cool", svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M4 12 a 8 8 0 0 1 14.5 -4.6"/>
                <path d="M20 12 a 8 8 0 0 1 -14.5 4.6"/>
                <path d="M14 7 H 19 V 2"/>
                <path d="M10 17 H 5 V 22"/>
              </svg>
            )},
            {name:"jack",  ch:"warm", svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1">
                <circle cx="12" cy="12" r="10"/>
                <circle cx="12" cy="12" r="6.5"/>
                <circle cx="12" cy="12" r="3" fill="currentColor" stroke="none"/>
                <path d="M12 2 V4 M12 20 V22 M2 12 H4 M20 12 H22"/>
              </svg>
            )},
            {name:"wave",  ch:"rose", svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round">
                <path d="M2 12 Q 5 2, 8 12 T 14 12 T 20 12 T 22 12"/>
                <path d="M2 16 Q 5 11, 8 16 T 14 16 T 20 16 T 22 16" opacity="0.45"/>
                <path d="M2 8  Q 5 3,  8 8  T 14 8  T 20 8  T 22 8"  opacity="0.45"/>
              </svg>
            )},
            {name:"gate",  ch:"life", svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinejoin="round" strokeLinecap="round">
                <path d="M2 17 V 9 H 8 V 17 H 12 V 13 H 18 V 17 H 22"/>
                <path d="M2 20 H 22" opacity="0.35"/>
                <circle cx="8"  cy="9"  r="0.9" fill="currentColor" stroke="none"/>
                <circle cx="18" cy="13" r="0.9" fill="currentColor" stroke="none"/>
              </svg>
            )},
            {name:"clock", ch:"amber",svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round">
                <circle cx="12" cy="12" r="10"/>
                <circle cx="12" cy="12" r="7"/>
                <path d="M12 12 V6.5 M12 12 L 16 14"/>
                <path d="M12 2.5 V4 M12 20 V21.5 M2.5 12 H4 M20 12 H21.5"/>
                <path d="M5 5 L 5.8 5.8 M19 19 L 18.2 18.2 M5 19 L 5.8 18.2 M19 5 L 18.2 5.8"/>
                <circle cx="12" cy="12" r="0.9" fill="currentColor" stroke="none"/>
              </svg>
            )},
            {name:"midi",  ch:"myth", svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1">
                <circle cx="12" cy="12" r="10"/>
                <path d="M5 16 A 9 9 0 0 1 19 16 Z" opacity="0.18" fill="currentColor" stroke="none"/>
                <circle cx="6.5"  cy="14.5" r="1.2" fill="currentColor" stroke="none"/>
                <circle cx="9"    cy="9"    r="1.2" fill="currentColor" stroke="none"/>
                <circle cx="12"   cy="7.2"  r="1.2" fill="currentColor" stroke="none"/>
                <circle cx="15"   cy="9"    r="1.2" fill="currentColor" stroke="none"/>
                <circle cx="17.5" cy="14.5" r="1.2" fill="currentColor" stroke="none"/>
                <path d="M5 16 A 9 9 0 0 1 19 16"/>
              </svg>
            )},
            {name:"audio", ch:"warm", svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round">
                <path d="M3  12 V 12"/>
                <path d="M6  10 V 14"/>
                <path d="M9  7  V 17"/>
                <path d="M12 3  V 21"/>
                <path d="M15 7  V 17"/>
                <path d="M18 10 V 14"/>
                <path d="M21 12 V 12"/>
              </svg>
            )},
            {name:"video", ch:"rose", svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round">
                <rect x="2" y="6" width="14" height="12" rx="1.2"/>
                <path d="M16 10 L 22 6.5 V 17.5 L 16 14 Z"/>
                <circle cx="6"  cy="9.5" r="0.7" fill="currentColor" stroke="none"/>
                <path d="M5 14 L 8 11.5 L 11 14 L 13 12.5" />
              </svg>
            )},
            {name:"node",  ch:"cool", svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round">
                <rect x="2"  y="9" width="7" height="6" rx="1"/>
                <rect x="15" y="9" width="7" height="6" rx="1"/>
                <path d="M9 12 H 15"/>
                <circle cx="9"  cy="12" r="0.9" fill="currentColor" stroke="none"/>
                <circle cx="15" cy="12" r="0.9" fill="currentColor" stroke="none"/>
                <path d="M4 11.5 H 7 M4 13 H 6" />
                <path d="M17 11.5 H 20 M18 13 H 20" />
              </svg>
            )},
            {name:"merge", ch:"axiom",svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M5 3 L 12 11 L 19 3"/>
                <path d="M12 11 V 21"/>
                <path d="M9 18 L 12 21 L 15 18"/>
                <circle cx="5"  cy="3" r="1.1" fill="currentColor" stroke="none"/>
                <circle cx="19" cy="3" r="1.1" fill="currentColor" stroke="none"/>
              </svg>
            )},
            {name:"split", ch:"axiom",svg:(
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 3 V 13"/>
                <path d="M5 21 L 12 13 L 19 21"/>
                <path d="M9 6 L 12 3 L 15 6"/>
                <circle cx="5"  cy="21" r="1.1" fill="currentColor" stroke="none"/>
                <circle cx="19" cy="21" r="1.1" fill="currentColor" stroke="none"/>
              </svg>
            )},
          ].map(g => (
            <IconNest key={g.name} title={g.name} colorVar={`--signal-${g.ch}`}>
              {g.svg}
            </IconNest>
          ))}
        </div>
      </section>

      {/* 12 — SPACING / SPEC ------------------------ */}
      <section className="section">
        <SectionHead num="12" title="Spec · Spacing & Density"
          blurb="VCV-derived measurement system. 1HP = 16px (5.08mm at our scale). Modules tile flush with no gap; the 4px gap between rack rows is structural, not spatial."/>
        <table className="spec-table">
          <thead><tr>
            <th>Token</th><th>Value</th><th>Use</th>
          </tr></thead>
          <tbody>
            <tr><td><code>--hp</code></td><td>16px</td><td>Module width unit · narrowest module = 4HP</td></tr>
            <tr><td><code>--s1..s20</code></td><td>4 / 8 / 12 / 16 / 20 / 24 / 32 / 40 / 48 / 64 / 80</td><td>Linear spacing scale</td></tr>
            <tr><td><code>--r-1..r-pill</code></td><td>2 / 4 / 6 / 8 / 12 / 16 / 999</td><td>Border-radii — cavities use r-3, modules r-4, screens r-3</td></tr>
            <tr><td><code>knob.size</code></td><td>40 / 48 / 64 / 68 px</td><td>S / M / L / Master</td></tr>
            <tr><td><code>jack.size</code></td><td>22px</td><td>Universal — includes rim</td></tr>
            <tr><td><code>fader.height</code></td><td>52 / 84 / 120 px</td><td>compact / standard / mix</td></tr>
            <tr><td><code>step.size</code></td><td>22px</td><td>Sequencer step button</td></tr>
            <tr><td><code>screen.padding</code></td><td>0px content · 4px label inset</td><td>Visualizers run edge-to-edge; engrave label sits on top</td></tr>
            <tr><td><code>module.gap</code></td><td>4px</td><td>Between modules in rack — emulates rail clip</td></tr>
            <tr><td><code>cable.thickness</code></td><td>2.2px stroke + 3.5px halo</td><td>1px traveling-dash overlay for signal animation</td></tr>
          </tbody>
        </table>
      </section>

      {/* CLOSER */}
      <footer style={{marginTop:96, paddingTop:32, borderTop:"1px solid rgba(255,255,255,0.05)", display:"flex", justifyContent:"space-between", fontFamily:"var(--font-mono)", fontSize:11, color:"var(--ink-dim)", letterSpacing:"0.1em", textTransform:"uppercase"}}>
        <div>TIMELINE · BIOSPARK STUDIOS · 2026</div>
        <div>v0.7 · PROCEDURAL RACK · 16-CH</div>
        <div style={{color:"var(--signal-life)", textShadow:"0 0 4px var(--signal-life)"}}>● ALL SYSTEMS LIVE</div>
      </footer>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(<App/>);
