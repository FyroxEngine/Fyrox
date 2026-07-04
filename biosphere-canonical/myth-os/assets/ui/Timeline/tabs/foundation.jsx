/* global React, Knob, Fader, Jack, Switch, IButton, LED, LEDLadder, SegmentDisplay, Screw, PatchWire, Scope, Spectrum, XYPad, VUMeter, Polar, CurveEditor, Spectrogram, ParticleField, Timeline, MIDIGrid, Waveform, channelVars */
const { useState: useStateF, useEffect: useEffectF, useRef: useRefF, useMemo: useMemoF } = React;

/* ═══════════════════════════════════════════════════════════════════
   FOUNDATION TAB — colors, type, signal types, glow channels
═══════════════════════════════════════════════════════════════════ */
function FoundationTab() {
  return (
    <div className="page">
      <div className="page-head">
        <div>
          <div className="page-eyebrow">DOC/00 · TOKENS</div>
          <h1 className="page-title">FOUNDATION</h1>
        </div>
        <div className="page-motto">"Every Patch Begins at the Void; Every Signal Earns its Light."</div>
      </div>

      <Section num="00.1" title="Color · Void Stack" sub="surfaces step from absolute void to inlay">
        <div className="grid" style={{gridTemplateColumns:'repeat(7, 1fr)'}}>
          {[
            ['void','#03050a','absolute void'],
            ['abyss','#07090f','deepest surface'],
            ['deep','#0d1117','page canvas'],
            ['surface','#111827','base card'],
            ['raised','#161d2e','elevated'],
            ['elevated','#1e293b','hover surface'],
            ['inlay','#243044','pressed inlay'],
          ].map(([n,h,desc]) => <Swatch key={n} name={n} hex={h} desc={desc}/>)}
        </div>
      </Section>

      <Section num="00.2" title="Color · Bioluminescent Channels" sub="strokes, glows, single-pixel ring motion">
        <div className="grid" style={{gridTemplateColumns:'repeat(6, 1fr)'}}>
          {[
            ['quantum','#00e5ff','data / flow'],
            ['bio','#39ff14','organic / alive'],
            ['mythos','#c084fc','arcane / narrative'],
            ['gold','#fbbf24','authority / forge'],
            ['ember','#f97316','heat / warning'],
            ['rose','#fb7185','tension / life'],
          ].map(([n,h,d]) => <GlowSwatch key={n} name={n} hex={h} desc={d}/>)}
        </div>
      </Section>

      <Section num="00.3" title="Signal Types" sub="every wire carries a typed meaning">
        <div className="grid" style={{gridTemplateColumns:'repeat(4, 1fr)'}}>
          {[
            {ch:'audio',  name:'AUDIO',  v:'±10V', desc:'Audio-rate signal · 48kHz / 24-bit'},
            {ch:'cv',     name:'CV',     v:'±5V',  desc:'Control voltage · param modulation'},
            {ch:'gate',   name:'GATE',   v:'0/+5V',desc:'Sustained boolean · note-on hold'},
            {ch:'trig',   name:'TRIG',   v:'pulse',desc:'Edge trigger · 1ms transient'},
            {ch:'clock',  name:'CLK',    v:'24ppq',desc:'Master tempo · phase lock'},
            {ch:'poly',   name:'POLY',   v:'×16',  desc:'Polyphonic bundle · 16 voices'},
            {ch:'midi',   name:'MIDI',   v:'14b',  desc:'MIDI 2.0 / MPE messages'},
            {ch:'stream', name:'STREAM', v:'tensor',desc:'Procedural data · n-dim tensor'},
          ].map(s => <SignalTypeCard key={s.ch} {...s}/>)}
        </div>
      </Section>

      <Section num="00.4" title="Type · Four Locked Families">
        <div className="grid" style={{gridTemplateColumns:'repeat(2, 1fr)', gap: 16}}>
          <TypeSpec name="DISPLAY · Cinzel Decorative" sample="THE FORGE IS LIT" font="var(--font-display)" sz={32} ls=".08em"/>
          <TypeSpec name="HEADER · Cinzel" sample="OPERATOR · TORNADO" font="var(--font-header)" sz={20} ls=".18em" tu/>
          <TypeSpec name="BODY · Space Grotesk" sample="Bind any concept to the rack. Patch its essence to a knob." font="var(--font-body)" sz={14} ls=".01em"/>
          <TypeSpec name="SCRIPT · Cormorant" sample={`"As Above, So Below, So Performed."`} font="var(--font-script)" sz={18} ls=".01em" italic/>
          <TypeSpec name="CODE · JetBrains Mono" sample="NODE/TORN.RT/v · 432.00hz · σ+2.4" font="var(--font-code)" sz={13} ls=".05em" code/>
        </div>
      </Section>

      <Section num="00.5" title="Spacing & Radii">
        <div className="row" style={{flexWrap:'wrap', gap: 24, alignItems:'flex-end'}}>
          {[4,8,12,16,20,24,32,48,64].map(s => (
            <div key={s} style={{display:'flex',flexDirection:'column',alignItems:'center',gap:6}}>
              <div style={{width:s, height:s, background:'var(--quantum)', boxShadow:'0 0 6px var(--quantum-glow)', borderRadius: 1}}/>
              <div className="cap-label">{s}</div>
            </div>
          ))}
        </div>
        <div style={{height:24}}/>
        <div className="row" style={{gap: 24, alignItems:'flex-end'}}>
          {[2,4,6,8,12,999].map(r => (
            <div key={r} style={{display:'flex',flexDirection:'column',alignItems:'center',gap:6}}>
              <div style={{width:48, height:48, background:'var(--panel-base)', borderRadius: r, border:'1px solid var(--border-lit)'}}/>
              <div className="cap-label">{r === 999 ? 'pill' : `${r}px`}</div>
            </div>
          ))}
        </div>
      </Section>

      <Section num="00.6" title="Glow Recipes" sub="middleground = light, never fill">
        <div className="grid" style={{gridTemplateColumns:'repeat(4, 1fr)', gap: 12}}>
          <GlowRecipe ch="audio" label="halo · audio"/>
          <GlowRecipe ch="cv" label="halo · cv"/>
          <GlowRecipe ch="gate" label="halo · gate"/>
          <GlowRecipe ch="trig" label="halo · trig"/>
        </div>
      </Section>
    </div>
  );
}
function Section({num, title, sub, children}) {
  return (
    <div className="section">
      <div className="section-head">
        <div className="section-num">§ {num}</div>
        <h2 className="section-title">{title}</h2>
        {sub && <div className="section-sub">{sub}</div>}
      </div>
      {children}
    </div>
  );
}
function Swatch({name,hex,desc}) {
  return (
    <div className="panel" style={{padding: 0, overflow:'hidden'}}>
      <div style={{height: 64, background: hex, borderBottom: '1px solid var(--border)'}}/>
      <div style={{padding: 10}}>
        <div style={{fontFamily:'var(--font-header)',fontSize:11,letterSpacing:'.18em',textTransform:'uppercase'}}>{name}</div>
        <div style={{fontFamily:'var(--font-code)',fontSize:10,color:'var(--quantum)',marginTop:2}}>{hex}</div>
        <div style={{fontFamily:'var(--font-script)',fontStyle:'italic',fontSize:11,color:'var(--fg-3)',marginTop:2}}>{desc}</div>
      </div>
    </div>
  );
}
function GlowSwatch({name,hex,desc}) {
  return (
    <div className="panel" style={{padding: 12, textAlign:'center'}}>
      <div style={{
        width: 56, height: 56, borderRadius:'50%',
        background: `radial-gradient(circle at 35% 30%, ${hex} 0%, ${hex}99 40%, #03050a 90%)`,
        boxShadow: `0 0 16px ${hex}66, inset 0 1px 2px rgba(255,255,255,0.3)`,
        margin: '8px auto', position: 'relative',
      }}>
        <div style={{
          position:'absolute',inset:-6, borderRadius:'50%',
          border: `1px solid ${hex}44`, animation: 'pulse-ring 3s ease-in-out infinite',
          ['--d-glow']: `${hex}55`,
        }}/>
      </div>
      <div style={{fontFamily:'var(--font-header)',fontSize:11,letterSpacing:'.2em',textTransform:'uppercase',marginTop:4}}>{name}</div>
      <div style={{fontFamily:'var(--font-code)',fontSize:9,color: hex,marginTop:2,textShadow:`0 0 4px ${hex}66`}}>{hex}</div>
      <div style={{fontFamily:'var(--font-script)',fontStyle:'italic',fontSize:10,color:'var(--fg-3)',marginTop:2}}>{desc}</div>
    </div>
  );
}
function SignalTypeCard({ch, name, v, desc}) {
  const c = channelVars(ch);
  return (
    <div className="panel" style={{padding: 14}}>
      <div style={{display:'flex',alignItems:'center',gap:10,marginBottom:10}}>
        <Jack channel={ch} dir="out" patched/>
        <svg width="60" height="20" viewBox="0 0 60 20">
          <path d="M 0 10 C 15 10, 15 4, 30 10 S 45 10, 60 10" fill="none" stroke={c.c} strokeWidth="2"
            style={{filter:`drop-shadow(0 0 3px ${c.g})`}}/>
          <path d="M 0 10 C 15 10, 15 4, 30 10 S 45 10, 60 10" fill="none" stroke="rgba(255,255,255,0.5)" strokeWidth="1"
            strokeDasharray="2 6" style={{animation:'wire-pulse 0.8s linear infinite'}}/>
        </svg>
        <Jack channel={ch} dir="in" patched/>
      </div>
      <div style={{display:'flex',justifyContent:'space-between',alignItems:'center',marginBottom:4}}>
        <div style={{fontFamily:'var(--font-header)',fontSize:12,letterSpacing:'.22em',color:c.c,textShadow:`0 0 4px ${c.g}`}}>{name}</div>
        <div style={{fontFamily:'var(--font-code)',fontSize:10,color:'var(--fg-3)',letterSpacing:'.1em'}}>{v}</div>
      </div>
      <div style={{fontFamily:'var(--font-body)',fontSize:11,color:'var(--fg-2)'}}>{desc}</div>
    </div>
  );
}
function TypeSpec({name, sample, font, sz, ls, italic, code, tu}) {
  return (
    <div className="panel" style={{padding: 16}}>
      <div className="cap-label" style={{marginBottom: 10}}>{name}</div>
      <div style={{
        fontFamily: font, fontSize: sz, letterSpacing: ls,
        fontStyle: italic ? 'italic' : 'normal',
        color: code ? 'var(--quantum)' : 'var(--fg-1)',
        textTransform: tu ? 'uppercase' : 'none',
        textShadow: code ? '0 0 4px var(--quantum-glow)' : 'none',
        lineHeight: 1.3,
      }}>{sample}</div>
    </div>
  );
}
function GlowRecipe({ch, label}) {
  const c = channelVars(ch);
  return (
    <div className="panel" style={{padding: 16, textAlign:'center'}}>
      <div style={{
        width: 48, height: 48, margin: '8px auto 14px', borderRadius: 4,
        background: 'var(--panel-base)',
        border: `1px solid ${c.c}`,
        boxShadow: `0 0 12px ${c.g}, 0 0 32px ${c.g}, inset 0 1px 0 rgba(255,255,255,0.06)`,
      }}/>
      <div className="cap-label">{label}</div>
      <code style={{fontSize:9,display:'block',marginTop:4,color:'var(--fg-3)'}}>0 0 12px {ch}-glow</code>
    </div>
  );
}

window.FoundationTab = FoundationTab;
window.Section = Section;
