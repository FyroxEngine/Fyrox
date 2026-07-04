/* global React, Knob, Fader, Jack, Switch, IButton, LED, LEDLadder, SegmentDisplay, Screw, Scope, Spectrum, XYPad, Polar, CurveEditor, MIDIGrid, Timeline, Waveform, VUMeter, Section, channelVars */
const { useState: useStateM } = React;

/* ═══════════════════════════════════════════════════════════════════
   MODULE — generic VCV-style rack panel (3HU)
═══════════════════════════════════════════════════════════════════ */
function RackModule({ width = 200, height = 320, title, code, accent = 'audio', children, motto }) {
  const c = channelVars(accent);
  return (
    <div style={{
      width, height,
      background: 'var(--panel-brushed)',
      border: '1px solid rgba(0,0,0,0.6)',
      borderRadius: 4,
      boxShadow: `
        inset 0 1px 0 rgba(255,255,255,0.07),
        0 4px 8px rgba(0,0,0,0.5),
        0 12px 28px rgba(0,0,0,0.4),
        0 0 0 1px ${c.c}22,
        inset 0 0 60px rgba(0,0,0,0.4)
      `,
      position: 'relative',
      display: 'flex',
      flexDirection: 'column',
      flexShrink: 0,
    }}>
      {/* top rail */}
      <div style={{
        display:'flex',justifyContent:'space-between',alignItems:'center',
        padding: '6px 8px',
        background: 'linear-gradient(180deg, rgba(255,255,255,0.04) 0%, transparent 100%)',
        borderBottom: `1px solid ${c.c}22`,
      }}>
        <Screw angle={45}/>
        <div style={{flex:1, textAlign:'center'}}>
          <div style={{
            fontFamily:'var(--font-header)',fontSize:10,
            letterSpacing:'.22em',color:c.c,
            textShadow:`0 0 4px ${c.g}`,
          }}>{title}</div>
        </div>
        <Screw angle={-30}/>
      </div>
      <div style={{flex:1, padding: 12, display:'flex', flexDirection:'column', gap: 10, overflow:'hidden'}}>
        {children}
      </div>
      {/* bottom rail */}
      <div style={{
        display:'flex',justifyContent:'space-between',alignItems:'center',
        padding: '4px 8px',
        background: 'linear-gradient(0deg, rgba(255,255,255,0.04) 0%, transparent 100%)',
        borderTop: `1px solid ${c.c}22`,
      }}>
        <Screw angle={20}/>
        <div style={{
          fontFamily:'var(--font-code)',fontSize:8,
          letterSpacing:'.18em',color:'var(--fg-3)',
        }}>{code}</div>
        <Screw angle={-15}/>
      </div>
      {motto && (
        <div style={{
          position:'absolute', bottom: 26, left: 0, right: 0,
          textAlign:'center',
          fontFamily:'var(--font-script)',fontStyle:'italic',
          fontSize:9, color:'var(--fg-3)', opacity: 0.7,
          padding: '0 12px',
        }}>{motto}</div>
      )}
    </div>
  );
}

function ModulesTab() {
  return (
    <div className="page">
      <div className="page-head">
        <div>
          <div className="page-eyebrow">DOC/03 · CHASSIS</div>
          <h1 className="page-title">MODULES</h1>
        </div>
        <div className="page-motto">"Every module is a vow: this surface, this signal, this consequence."</div>
      </div>

      <Section num="03.1" title="The Module · 3HU Pattern" sub="rails · screws · top-anchored title · bottom code">
        <div className="panel" style={{padding: 28, background:'var(--abyss)', display:'flex',justifyContent:'center'}}>
          <RackModule title="OPERATOR" code="OPR/v3.1" accent="audio">
            <Knob size={56} value={0.62} variant="indexed" channel="audio" label="FREQ" min={20} max={20000} unit="Hz" readout="432.0"/>
            <div style={{display:'flex',gap:10,justifyContent:'center'}}>
              <Knob size={36} value={0.5} variant="classic" channel="cv" label="FM" bipolar/>
              <Knob size={36} value={0.7} variant="classic" channel="cv" label="AM"/>
            </div>
            <Switch options={['SIN','SAW','TRI','SQR']} channel="audio" label="WAVE" value={1}/>
            <div style={{flex:1}}/>
            <div style={{display:'flex',justifyContent:'space-around'}}>
              <Jack channel="cv" dir="in" label="V/OCT"/>
              <Jack channel="cv" dir="in" label="FM"/>
              <Jack channel="audio" dir="out" patched label="OUT"/>
            </div>
          </RackModule>
        </div>
      </Section>

      <Section num="03.2" title="Module Catalog" sub="six archetypes · combine into rack rows">
        <div className="panel" style={{padding: 24, background:'var(--abyss)', overflow:'auto'}}>
          <div style={{display:'flex',gap:8,minWidth:'min-content'}}>
            <RackModule title="OPERATOR" code="OPR/v3.1" accent="audio">
              <Knob size={56} value={0.6} variant="indexed" channel="audio" label="FREQ" readout="432.0"/>
              <div style={{display:'flex',gap:10,justifyContent:'center'}}>
                <Knob size={32} value={0.5} variant="classic" channel="cv" label="FM" bipolar/>
                <Knob size={32} value={0.7} variant="classic" channel="cv" label="AM"/>
              </div>
              <Switch options={['SIN','SAW','TRI','SQR']} channel="audio" value={1}/>
              <div style={{flex:1}}/>
              <div style={{display:'flex',justifyContent:'space-around'}}>
                <Jack channel="cv" dir="in" label="V/O"/>
                <Jack channel="cv" dir="in" label="FM"/>
                <Jack channel="audio" dir="out" patched label="OUT"/>
              </div>
            </RackModule>

            <RackModule title="ENVELOPE" code="ENV/v2.4" accent="cv" width={180}>
              <CurveEditor preset="adsr" channel="cv" width={150} height={70}/>
              <div style={{display:'grid',gridTemplateColumns:'repeat(4,1fr)',gap:6,justifyItems:'center'}}>
                <Knob size={28} value={0.2} variant="classic" channel="cv" label="A" readout={false}/>
                <Knob size={28} value={0.4} variant="classic" channel="cv" label="D" readout={false}/>
                <Knob size={28} value={0.6} variant="classic" channel="cv" label="S" readout={false}/>
                <Knob size={28} value={0.5} variant="classic" channel="cv" label="R" readout={false}/>
              </div>
              <div style={{flex:1}}/>
              <div style={{display:'flex',justifyContent:'space-around'}}>
                <Jack channel="trig" dir="in" patched label="GATE"/>
                <Jack channel="cv" dir="out" patched label="ENV"/>
              </div>
            </RackModule>

            <RackModule title="LFO" code="LFO/v1.2" accent="mythos">
              <Polar petals={5} channel="mythos" size={110}/>
              <div style={{display:'flex',gap:10,justifyContent:'center'}}>
                <Knob size={36} value={0.25} variant="indexed" channel="mythos" label="RATE" readout="2.5"/>
                <Knob size={36} value={0.55} variant="classic" channel="mythos" label="SHAPE"/>
              </div>
              <div style={{flex:1}}/>
              <div style={{display:'flex',justifyContent:'space-around'}}>
                <Jack channel="clock" dir="in" label="SYNC"/>
                <Jack channel="cv" dir="out" patched label="OUT"/>
                <Jack channel="cv" dir="out" label="QUAD"/>
              </div>
            </RackModule>

            <RackModule title="SCOPE" code="SCP/v2.0" accent="bio" width={220}>
              <Scope width={196} height={80} channel="bio" source="fm"/>
              <Spectrum width={196} height={50} bars={28} channel="bio"/>
              <div style={{display:'flex',gap:8,justifyContent:'center'}}>
                <Knob size={28} value={0.5} variant="classic" channel="bio" label="TIME" readout={false}/>
                <Knob size={28} value={0.7} variant="classic" channel="bio" label="GAIN" readout={false}/>
                <SegmentDisplay text="48k" channel="bio" height={20}/>
              </div>
              <div style={{flex:1}}/>
              <div style={{display:'flex',justifyContent:'space-around'}}>
                <Jack channel="audio" dir="in" patched label="IN A"/>
                <Jack channel="audio" dir="in" label="IN B"/>
                <Jack channel="cv" dir="out" label="TRIG"/>
              </div>
            </RackModule>

            <RackModule title="MIXER" code="MIX/v4.0" accent="gold" width={140}>
              <div style={{display:'flex',gap:8,justifyContent:'center',alignItems:'flex-end'}}>
                <Fader value={0.7} length={100} channel="audio" readout="-2"/>
                <Fader value={0.55} length={100} channel="audio" readout="-6"/>
                <Fader value={0.4} length={100} channel="audio" readout="-9"/>
              </div>
              <div style={{display:'flex',gap:6,justifyContent:'center'}}>
                <Knob size={22} value={0.5} variant="classic" channel="rose" label="P" readout={false}/>
                <Knob size={22} value={0.4} variant="classic" channel="rose" label="P" readout={false}/>
                <Knob size={22} value={0.6} variant="classic" channel="rose" label="P" readout={false}/>
              </div>
              <div style={{flex:1}}/>
              <div style={{display:'flex',justifyContent:'space-around'}}>
                <Jack channel="audio" dir="in" patched label="1"/>
                <Jack channel="audio" dir="in" patched label="2"/>
                <Jack channel="audio" dir="in" label="3"/>
              </div>
              <div style={{display:'flex',justifyContent:'center',marginTop:4}}>
                <Jack channel="audio" dir="out" patched label="SUM"/>
              </div>
            </RackModule>

            <RackModule title="SEQUENCER" code="SEQ/v5.2" accent="trig" width={260}>
              <MIDIGrid width={236} height={60} channel="midi"/>
              <div style={{display:'grid',gridTemplateColumns:'repeat(8,1fr)',gap:4}}>
                {[0,1,2,3,4,5,6,7].map(i => (
                  <div key={i} style={{display:'flex',flexDirection:'column',alignItems:'center',gap:3}}>
                    <Knob size={22} value={0.2+i*0.1} variant="classic" channel="trig" readout={false}/>
                    <LED on={i!==2 && i!==5} channel="trig"/>
                  </div>
                ))}
              </div>
              <div style={{flex:1}}/>
              <div style={{display:'flex',justifyContent:'space-between',padding:'0 8px'}}>
                <Jack channel="clock" dir="in" patched label="CLK"/>
                <Switch options={['FWD','REV','RND']} channel="trig" value={0}/>
                <Jack channel="cv" dir="out" patched label="V/O"/>
                <Jack channel="trig" dir="out" patched label="TRG"/>
              </div>
            </RackModule>
          </div>
        </div>
      </Section>

      <Section num="03.3" title="Node Graph · Houdini-style" sub="operator chain · typed wires">
        <NodeGraph/>
      </Section>

      <Section num="03.4" title="Channel Box · Maya-style" sub="per-attribute parameter list with curves">
        <ChannelBox/>
      </Section>
    </div>
  );
}

/* ─── NodeGraph — Houdini-style ─────────────────────────────── */
function NodeGraph() {
  const nodes = [
    {id:'src',  x:40,  y:60,  title:'TORNADO_SRC', sub:'particle/v1', color:'mythos', io: {in: ['seed'], out: ['pts']}},
    {id:'noise',x:240, y:30,  title:'CURL_NOISE',  sub:'vop/v2',     color:'cv',     io: {in: ['pts','time'], out: ['vel']}},
    {id:'drv',  x:240, y:160, title:'DRIVER_OSC',  sub:'audio/v3',   color:'audio',  io: {in: ['v/oct'], out: ['hz']}},
    {id:'mix',  x:480, y:90,  title:'CHAOS_MIX',   sub:'attribvop',  color:'gold',   io: {in: ['vel','hz'], out: ['out']}},
    {id:'rndr', x:720, y:90,  title:'RENDER',      sub:'output/v1',  color:'bio',    io: {in: ['out'], out: []}},
  ];
  const wires = [
    {from:'src.pts', to:'noise.pts', ch:'mythos'},
    {from:'noise.vel', to:'mix.vel', ch:'cv'},
    {from:'drv.hz', to:'mix.hz', ch:'audio'},
    {from:'mix.out', to:'rndr.out', ch:'gold'},
  ];
  const W = 880, H = 260;

  // resolve port positions
  const nodeBoxW = 180, nodeBoxH = 100;
  const portPos = (ref) => {
    const [nid, port] = ref.split('.');
    const n = nodes.find(x => x.id === nid);
    const isIn = n.io.in.includes(port);
    const list = isIn ? n.io.in : n.io.out;
    const idx = list.indexOf(port);
    const xRel = isIn ? 0 : nodeBoxW;
    const yRel = 36 + idx * 18 + 8;
    return { x: n.x + xRel, y: n.y + yRel };
  };

  return (
    <div className="panel" style={{padding: 0, overflow:'hidden', position:'relative'}}>
      <div style={{
        background: 'var(--abyss)', position: 'relative',
        backgroundImage:
          'linear-gradient(rgba(120,180,255,0.04) 1px, transparent 1px), linear-gradient(90deg, rgba(120,180,255,0.04) 1px, transparent 1px)',
        backgroundSize: '20px 20px',
        height: H, width: '100%', minWidth: W,
      }}>
        <svg width={W} height={H} style={{position:'absolute',inset:0,overflow:'visible'}}>
          {wires.map((w,i) => {
            const f = portPos(w.from), t = portPos(w.to);
            const c = channelVars(w.ch);
            const dx = (t.x - f.x) / 2;
            const path = `M ${f.x} ${f.y} C ${f.x+dx} ${f.y}, ${t.x-dx} ${t.y}, ${t.x} ${t.y}`;
            return (
              <g key={i}>
                <path d={path} fill="none" stroke="#000" strokeWidth="4" opacity="0.6"/>
                <path d={path} fill="none" stroke={c.c} strokeWidth="2"
                  style={{filter:`drop-shadow(0 0 4px ${c.g})`}}/>
                <path d={path} fill="none" stroke="rgba(255,255,255,0.5)" strokeWidth="1"
                  strokeDasharray="2 8" style={{animation:'wire-pulse 1s linear infinite'}}/>
              </g>
            );
          })}
        </svg>
        {nodes.map(n => {
          const c = channelVars(n.color);
          return (
            <div key={n.id} style={{
              position:'absolute', left: n.x, top: n.y,
              width: nodeBoxW,
              background: 'var(--panel-base)',
              border: `1px solid ${c.c}55`,
              borderRadius: 3,
              boxShadow: `0 4px 12px rgba(0,0,0,0.5), 0 0 16px ${c.g}, inset 0 1px 0 rgba(255,255,255,0.06)`,
            }}>
              <div style={{
                padding:'4px 8px',
                background: `linear-gradient(180deg, ${c.c}15 0%, transparent 100%)`,
                borderBottom: `1px solid ${c.c}30`,
                display:'flex',justifyContent:'space-between',alignItems:'center',
              }}>
                <div style={{
                  fontFamily:'var(--font-header)',fontSize:10,letterSpacing:'.18em',
                  color:c.c, textShadow:`0 0 4px ${c.g}`,
                }}>{n.title}</div>
                <div style={{fontFamily:'var(--font-code)',fontSize:8,color:'var(--fg-3)'}}>{n.sub}</div>
              </div>
              <div style={{display:'flex', position: 'relative', minHeight: 60}}>
                <div style={{flex:1, padding:'4px 8px'}}>
                  {n.io.in.map((p,i) => (
                    <div key={p} style={{display:'flex',alignItems:'center',gap:6,fontSize:9,fontFamily:'var(--font-code)',color:'var(--fg-2)',height:18}}>
                      <div style={{width:8,height:8,borderRadius:'50%',background:c.c,boxShadow:`0 0 4px ${c.g}`}}/>
                      {p}
                    </div>
                  ))}
                </div>
                <div style={{flex:1, padding:'4px 8px', textAlign:'right'}}>
                  {n.io.out.map((p,i) => (
                    <div key={p} style={{display:'flex',alignItems:'center',justifyContent:'flex-end',gap:6,fontSize:9,fontFamily:'var(--font-code)',color:'var(--fg-2)',height:18}}>
                      {p}
                      <div style={{width:8,height:8,borderRadius:'50%',background:c.c,boxShadow:`0 0 4px ${c.g}`}}/>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

/* ─── ChannelBox — Maya-style ─────────────────────────────── */
function ChannelBox() {
  const rows = [
    {name:'translateX', val:'  2.418', anim:true,  ch:'cv'},
    {name:'translateY', val:'  0.000', anim:false, ch:'cv'},
    {name:'translateZ', val:'-12.530', anim:true,  ch:'cv'},
    {name:'rotateY',    val:'127.000', anim:true,  ch:'mythos'},
    {name:'scaleU',     val:'  1.000', anim:false, ch:'gold'},
    {name:'turbulence', val:'  0.640', anim:true,  ch:'mythos'},
    {name:'driveHz',    val:'432.000', anim:true,  ch:'audio'},
    {name:'voicePoly',  val:' 16',     anim:false, ch:'poly'},
    {name:'aliveness',  val:'  0.842', anim:true,  ch:'bio'},
  ];
  return (
    <div className="panel" style={{padding: 0, overflow:'hidden'}}>
      <div style={{display:'grid',gridTemplateColumns:'1fr 1fr'}}>
        <div style={{borderRight:'1px solid var(--border)'}}>
          <div style={{padding:'8px 14px',background:'rgba(255,255,255,0.015)',borderBottom:'1px solid var(--border)'}}>
            <div className="cap-label">CHANNEL BOX · TORNADO_01</div>
          </div>
          {rows.map((r,i) => {
            const c = channelVars(r.ch);
            return (
              <div key={i} style={{
                display:'grid',gridTemplateColumns:'14px 1fr auto 60px',
                gap: 8, padding:'5px 12px', alignItems:'center',
                background: i%2 ? 'rgba(255,255,255,0.012)' : 'transparent',
                borderBottom: '1px solid rgba(120,180,255,0.04)',
              }}>
                <div style={{
                  width:8,height:8,borderRadius:'50%',
                  background: r.anim ? c.c : 'var(--led-off)',
                  boxShadow: r.anim ? `0 0 6px ${c.g}` : 'inset 0 1px 1px rgba(0,0,0,0.5)',
                }}/>
                <div style={{fontFamily:'var(--font-body)',fontSize:11,color:'var(--fg-1)'}}>{r.name}</div>
                <div style={{fontFamily:'var(--font-code)',fontSize:11,color:c.c,textShadow:r.anim?`0 0 3px ${c.g}`:'none'}}>{r.val}</div>
                <div style={{height:14}}>
                  {r.anim && <SparkLine ch={r.ch} seed={i}/>}
                </div>
              </div>
            );
          })}
        </div>
        <div style={{padding:14, display:'flex',flexDirection:'column',gap:14}}>
          <div className="cap-label">PARAMETER · TURBULENCE</div>
          <CurveEditor preset="lfo" channel="mythos" width={300} height={120}/>
          <div style={{display:'flex',gap:8,justifyContent:'space-between',alignItems:'center'}}>
            <div className="cap-label">KEYFRAMES</div>
            <Timeline width={300} channel="gold" frames={48}/>
          </div>
          <div className="motto" style={{fontSize:11,color:'var(--fg-3)'}}>"A keyframe is a vow recorded against the void."</div>
        </div>
      </div>
    </div>
  );
}
function SparkLine({ch='cv', seed=0}) {
  const c = channelVars(ch);
  const pts = [];
  for (let i=0;i<20;i++) {
    pts.push(`${i*3},${7 - Math.sin(i*0.6 + seed)*5}`);
  }
  return (
    <svg width={60} height={14}>
      <polyline points={pts.join(' ')} fill="none" stroke={c.c} strokeWidth="1"
        style={{filter:`drop-shadow(0 0 2px ${c.g})`}}/>
    </svg>
  );
}

window.ModulesTab = ModulesTab;
window.RackModule = RackModule;
