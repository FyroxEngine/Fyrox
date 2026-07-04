/* global React, Scope, Spectrum, XYPad, VUMeter, Polar, CurveEditor, Spectrogram, ParticleField, Timeline, MIDIGrid, Waveform, Knob, Section */

function VisualizersTab() {
  return (
    <div className="page">
      <div className="page-head">
        <div>
          <div className="page-eyebrow">DOC/02 · GLASS</div>
          <h1 className="page-title">VISUALIZERS</h1>
        </div>
        <div className="page-motto">"The screen is a scrying-stone. What it shows, the rack already knows."</div>
      </div>

      <Section num="02.1" title="Oscilloscopes" sub="time-domain · sliding waveforms">
        <div className="grid" style={{gridTemplateColumns:'repeat(2, 1fr)', gap: 16}}>
          <VizCard label="SINE · 432 Hz" tag="SCOPE/01"><Scope source="sine" channel="audio"/></VizCard>
          <VizCard label="FM · 2-OP" tag="SCOPE/02"><Scope source="fm" channel="bio"/></VizCard>
          <VizCard label="NOISY SAW" tag="SCOPE/03"><Scope source="noise" channel="ember"/></VizCard>
          <VizCard label="HARMONIC" tag="SCOPE/04"><Scope source="harm" channel="mythos"/></VizCard>
        </div>
      </Section>

      <Section num="02.2" title="Spectrum & Spectrogram">
        <div className="grid" style={{gridTemplateColumns:'repeat(2, 1fr)', gap: 16}}>
          <VizCard label="FFT · 32 BIN" tag="FFT/01"><Spectrum bars={32} channel="poly"/></VizCard>
          <VizCard label="FFT · 64 BIN · CYAN" tag="FFT/02"><Spectrum bars={64} channel="audio"/></VizCard>
          <VizCard label="SPECTROGRAM · t→ν" tag="SGM/01"><Spectrogram/></VizCard>
          <VizCard label="WAVEFORM · BUFFER" tag="WAV/01"><Waveform channel="quantum"/></VizCard>
        </div>
      </Section>

      <Section num="02.3" title="Phase · Polar · Field">
        <div style={{display:'flex',gap:16,flexWrap:'wrap',justifyContent:'center'}}>
          <VizCard label="XY PHASE" tag="XY/01"><XYPad channel="mythos"/></VizCard>
          <VizCard label="POLAR · 7-FOLD" tag="POL/01"><Polar petals={7} channel="gold"/></VizCard>
          <VizCard label="POLAR · 11-FOLD" tag="POL/02"><Polar petals={11} channel="rose" size={140}/></VizCard>
          <VizCard label="VORTEX FIELD" tag="VFX/01"><ParticleField channel="mythos"/></VizCard>
        </div>
      </Section>

      <Section num="02.4" title="Meters · VU Bay">
        <div className="panel" style={{padding: 18, display:'flex',flexDirection:'column',gap:8}}>
          <VUMeter channel="bio" label="L"/>
          <VUMeter channel="bio" label="R"/>
          <VUMeter channel="cv" label="MOD"/>
          <VUMeter channel="gold" label="SUM"/>
          <VUMeter channel="rose" label="SC"/>
          <VUMeter channel="ember" label="DRV"/>
        </div>
      </Section>

      <Section num="02.5" title="Curve Editors · ADSR / LFO">
        <div className="grid" style={{gridTemplateColumns:'repeat(2, 1fr)', gap: 16}}>
          <VizCard label="ENVELOPE · ADSR" tag="ENV/01"><CurveEditor preset="adsr" channel="cv"/></VizCard>
          <VizCard label="LFO · SINE 2.5Hz" tag="LFO/01"><CurveEditor preset="lfo" channel="mythos"/></VizCard>
        </div>
      </Section>

      <Section num="02.6" title="Timeline · Piano Roll">
        <div className="panel" style={{padding: 18, display:'flex',flexDirection:'column',gap:14}}>
          <Timeline width={520} channel="gold"/>
          <MIDIGrid width={520} height={100} channel="midi"/>
        </div>
      </Section>
    </div>
  );
}

function VizCard({label, tag, children}) {
  return (
    <div className="panel" style={{padding: 14}}>
      <div style={{display:'flex',justifyContent:'space-between',alignItems:'center',marginBottom:10}}>
        <div className="cap-label">{label}</div>
        <div style={{fontFamily:'var(--font-code)',fontSize:9,color:'var(--gold)',letterSpacing:'.15em'}}>{tag}</div>
      </div>
      <div style={{display:'flex',justifyContent:'center'}}>{children}</div>
    </div>
  );
}

window.VisualizersTab = VisualizersTab;
