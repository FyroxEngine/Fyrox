/* global React, Knob, Fader, Jack, Switch, IButton, LED, LEDLadder, SegmentDisplay, Screw, Section */
const { useState: useStateA } = React;

function AtomsTab() {
  return (
    <div className="page">
      <div className="page-head">
        <div>
          <div className="page-eyebrow">DOC/01 · ATOMS</div>
          <h1 className="page-title">CONTROLS</h1>
        </div>
        <div className="page-motto">"Each atom is a covenant: a hand on a parameter, a parameter on the world."</div>
      </div>

      <Section num="01.1" title="Knobs · Five Silhouettes" sub="drag vertically to rotate · 270° sweep">
        <div className="panel" style={{padding: 32}}>
          <div style={{display:'grid', gridTemplateColumns:'repeat(5, 1fr)', gap: 24, alignItems:'end', justifyItems:'center'}}>
            <KnobShowcase variant="classic" channel="audio" label="CLASSIC"/>
            <KnobShowcase variant="ringed" channel="cv" label="RINGED"/>
            <KnobShowcase variant="indexed" channel="gate" label="INDEXED"/>
            <KnobShowcase variant="hatched" channel="trig" label="HATCHED"/>
            <KnobShowcase variant="gilded" channel="gold" label="GILDED" gold/>
          </div>
        </div>
      </Section>

      <Section num="01.2" title="Knob Sizes · 28 → 80" sub="density is a feature">
        <div className="panel" style={{padding: 28}}>
          <div style={{display:'flex',gap:32,alignItems:'flex-end',justifyContent:'center'}}>
            {[28, 36, 48, 64, 80].map(s => (
              <div key={s} style={{display:'flex',flexDirection:'column',alignItems:'center',gap:8}}>
                <Knob size={s} value={0.42 + s/200} variant="classic" channel="audio" readout={false}/>
                <div className="cap-label">{s}px</div>
              </div>
            ))}
          </div>
        </div>
      </Section>

      <Section num="01.3" title="Bipolar · Modulation knobs">
        <div className="panel" style={{padding: 24}}>
          <div style={{display:'flex',gap:32,justifyContent:'center'}}>
            <Knob value={0.62} variant="indexed" bipolar channel="cv" label="MOD" min={-1} max={1} unit="V" size={56}/>
            <Knob value={0.5} variant="indexed" bipolar channel="mythos" label="DETUNE" min={-12} max={12} unit="ct" size={56}/>
            <Knob value={0.35} variant="ringed" bipolar channel="rose" label="PAN" min={-1} max={1} size={56}/>
            <Knob value={0.78} variant="indexed" bipolar channel="ember" label="FB" min={-1} max={1} size={56}/>
          </div>
        </div>
      </Section>

      <Section num="01.4" title="Faders" sub="vertical · horizontal · LED-tracked">
        <div className="panel" style={{padding: 28}}>
          <div style={{display:'flex',gap:48,justifyContent:'center',alignItems:'flex-end'}}>
            <Fader value={0.7} channel="audio" length={140} label="GAIN" readout="-2.4 dB"/>
            <Fader value={0.55} channel="cv" length={140} label="MIX" readout="55%"/>
            <Fader value={0.85} channel="poly" length={140} label="LEVEL" readout="+1.1"/>
            <Fader value={0.42} channel="rose" length={140} label="WET" readout="42%"/>
            <div style={{display:'flex',flexDirection:'column',gap:14, marginLeft: 24}}>
              <Fader value={0.6} channel="quantum" length={180} orient="h" label="CUTOFF"/>
              <Fader value={0.32} channel="mythos" length={180} orient="h" label="RESONANCE"/>
              <Fader value={0.48} channel="bio" length={180} orient="h" label="DRIFT"/>
            </div>
          </div>
        </div>
      </Section>

      <Section num="01.5" title="Jacks · Inputs / Outputs" sub="ring color = signal type · solid = output · ringed = input">
        <div className="panel" style={{padding: 24}}>
          <div style={{display:'grid', gridTemplateColumns:'repeat(8, 1fr)', gap: 20, justifyItems:'center'}}>
            <Jack channel="audio" dir="out" patched label="OUT"/>
            <Jack channel="audio" dir="in" label="IN"/>
            <Jack channel="cv" dir="out" patched label="CV"/>
            <Jack channel="cv" dir="in" label="MOD"/>
            <Jack channel="gate" dir="out" patched label="GATE"/>
            <Jack channel="trig" dir="in" patched label="TRG"/>
            <Jack channel="poly" dir="out" patched label="POLY"/>
            <Jack channel="midi" dir="in" label="MIDI"/>
          </div>
        </div>
      </Section>

      <Section num="01.6" title="Switches · 2 / 3 / 5-way">
        <div className="panel" style={{padding: 24}}>
          <div style={{display:'flex',gap:32,justifyContent:'center',alignItems:'center'}}>
            <Switch options={['OFF','ON']} channel="bio" label="POWER" value={1}/>
            <Switch options={['SIN','SAW','TRI']} channel="audio" label="WAVE" value={1}/>
            <Switch options={['1','2','4','8','16']} channel="gold" label="DIV" value={2}/>
            <Switch options={['SLOW','FAST']} channel="rose" label="RATE" orient="h" value={0}/>
            <Switch options={['LIVE','REC','LOOP']} channel="trig" label="MODE" orient="h" value={0}/>
          </div>
        </div>
      </Section>

      <Section num="01.7" title="Buttons & LEDs">
        <div className="panel" style={{padding: 24}}>
          <div style={{display:'flex',gap:36,justifyContent:'center',alignItems:'flex-start'}}>
            <div style={{display:'flex',gap:14}}>
              <IButton on channel="bio" icon="▶" label="RUN" size={36}/>
              <IButton channel="ember" icon="●" label="REC" size={36}/>
              <IButton channel="gold" icon="◼" label="STOP" size={36}/>
              <IButton on channel="quantum" icon="↻" label="RESET" size={36}/>
            </div>
            <div style={{display:'flex',flexDirection:'column',gap:8,alignItems:'center'}}>
              <div style={{display:'flex',gap:8}}>
                <LED on channel="bio" blink/>
                <LED on channel="gold"/>
                <LED on channel="ember" blink/>
                <LED channel="rose"/>
                <LED on channel="quantum"/>
                <LED on channel="mythos" blink/>
              </div>
              <div className="cap-label">CHAIN · 6</div>
            </div>
            <div style={{display:'flex',gap:14,alignItems:'flex-end'}}>
              <LEDLadder count={14} level={0.7} channel="bio"/>
              <LEDLadder count={14} level={0.92} channel="bio"/>
              <LEDLadder count={14} level={0.45} channel="bio"/>
              <LEDLadder count={14} level={0.81} channel="bio"/>
            </div>
          </div>
        </div>
      </Section>

      <Section num="01.8" title="Numeric Displays">
        <div className="panel" style={{padding: 24}}>
          <div style={{display:'flex',gap:18,flexWrap:'wrap',alignItems:'center',justifyContent:'center'}}>
            <SegmentDisplay text="432.00" channel="ember" height={32}/>
            <SegmentDisplay text="120 BPM" channel="gold" height={32}/>
            <SegmentDisplay text="C4" channel="bio" height={32}/>
            <SegmentDisplay text="0:42:18" channel="quantum" height={32}/>
            <SegmentDisplay text="-12.3" channel="rose" height={32}/>
            <SegmentDisplay text="POLY 16" channel="mythos" height={32}/>
          </div>
        </div>
      </Section>

      <Section num="01.9" title="Hardware Trim · Screws & Rails">
        <div className="panel" style={{padding: 0, overflow:'hidden'}}>
          <div style={{
            background:'var(--panel-rail)',
            padding: 8, display:'flex', justifyContent:'space-between',
            borderBottom: '1px solid rgba(0,0,0,0.5)',
          }}>
            <div style={{display:'flex',gap:8,alignItems:'center'}}>
              <Screw angle={45}/><Screw angle={-30}/>
              <span className="cap-label" style={{marginLeft:8}}>RAIL · TOP · 1HU</span>
            </div>
            <div style={{display:'flex',gap:8,alignItems:'center'}}>
              <span style={{fontFamily:'var(--font-code)',fontSize:9,color:'var(--fg-3)',letterSpacing:'.18em'}}>BSP-AETHYR-001</span>
              <Screw angle={15}/><Screw angle={70}/>
            </div>
          </div>
          <div style={{padding: 32, display:'flex',justifyContent:'center',alignItems:'center',gap: 28}}>
            <Knob value={0.6} variant="classic" channel="audio" label="CHASSIS" size={48}/>
            <Knob value={0.4} variant="indexed" channel="gold" label="EXAMPLE" size={48}/>
            <div className="motto" style={{maxWidth:200, fontSize: 13}}>"From mark, meaning. From meaning, mandate."</div>
          </div>
          <div style={{
            background:'var(--panel-rail)',
            padding: 8, display:'flex', justifyContent:'space-between',
            borderTop: '1px solid rgba(0,0,0,0.5)',
          }}>
            <Screw angle={20}/><Screw angle={-50}/>
          </div>
        </div>
      </Section>
    </div>
  );
}

function KnobShowcase({variant, channel, label, gold}) {
  return (
    <div style={{display:'flex',flexDirection:'column',alignItems:'center',gap:10}}>
      <Knob size={64} value={0.62} variant={variant} channel={channel} readout={false}/>
      <div className="cap-label" style={{color: gold ? 'var(--gold)' : 'var(--fg-2)', textShadow: gold ? '0 0 6px var(--gold-glow)' : 'none'}}>{label}</div>
    </div>
  );
}

window.AtomsTab = AtomsTab;
