/* =====================================================
   PATCH — the Tornado rack with live cables + node graph + timeline
   ===================================================== */

const { useState: usePState, useEffect: usePEffect, useMemo: usePMemo, useRef: usePRef } = React;

/* ---------- live patch cables overlay ---------- */
function PatchCables({ rackRef, cables }) {
  const [, force] = usePState(0);
  usePEffect(() => {
    const onResize = () => force(n => n + 1);
    window.addEventListener("resize", onResize);
    const t = setTimeout(onResize, 50);
    return () => { window.removeEventListener("resize", onResize); clearTimeout(t); };
  }, []);

  const [paths, setPaths] = usePState([]);
  usePEffect(() => {
    if (!rackRef.current) return;
    const compute = () => {
      const rackRect = rackRef.current.getBoundingClientRect();
      const found = cables.map(({ from, to, color }) => {
        const a = rackRef.current.querySelector(`[data-jack="${from}"]`);
        const b = rackRef.current.querySelector(`[data-jack="${to}"]`);
        if (!a || !b) return null;
        const ar = a.getBoundingClientRect();
        const br = b.getBoundingClientRect();
        return {
          x1: ar.left - rackRect.left + ar.width/2,
          y1: ar.top  - rackRect.top  + ar.height/2,
          x2: br.left - rackRect.left + br.width/2,
          y2: br.top  - rackRect.top  + br.height/2,
          color,
        };
      }).filter(Boolean);
      setPaths(found);
    };
    compute();
    const t = setTimeout(compute, 100);
    const t2 = setTimeout(compute, 400);
    return () => { clearTimeout(t); clearTimeout(t2); };
  }, [cables, rackRef]);

  return (
    <svg className="patch-cables" style={{ position:"absolute", inset:0, pointerEvents:"none", zIndex: 5}}>
      <defs>
        <filter id="cable-glow" x="-20%" y="-20%" width="140%" height="140%">
          <feGaussianBlur stdDeviation="2" />
          <feMerge><feMergeNode/><feMergeNode in="SourceGraphic"/></feMerge>
        </filter>
      </defs>
      {paths.map((p, i) => {
        const dx = p.x2 - p.x1;
        const sag = 28 + Math.abs(dx) * 0.18;
        const my = (p.y1 + p.y2) / 2 + sag;
        const d = `M ${p.x1} ${p.y1} Q ${(p.x1+p.x2)/2} ${my} ${p.x2} ${p.y2}`;
        return (
          <g key={i}>
            <path d={d} stroke={p.color} strokeWidth="3.5" fill="none"
                  opacity="0.35" filter="url(#cable-glow)"/>
            <path d={d} stroke={p.color} strokeWidth="2.2" fill="none" strokeLinecap="round"/>
            <path d={d} stroke="rgba(255,255,255,0.6)" strokeWidth="0.6" fill="none" strokeDasharray="2 6"
                  opacity="0.5">
              <animate attributeName="stroke-dashoffset" from="0" to="-16" dur="0.8s" repeatCount="indefinite"/>
            </path>
            <circle cx={p.x1} cy={p.y1} r="3" fill={p.color} opacity="0.9"/>
            <circle cx={p.x2} cy={p.y2} r="3" fill={p.color} opacity="0.9"/>
          </g>
        );
      })}
    </svg>
  );
}

/* ---------- patched module wrappers — adds data-jack attrs ---------- */
function withJackTags(node, tagMap) {
  // walk children, find jacks by label and add data-jack
  // simpler: we'll use ref-based approach via an effect.
  return node;
}

/* simpler: I'll re-build the modules inline here with data-jack ids,
   but to keep it DRY, we just stamp ids after mount: */
function useStampJacks(rackRef, mapping) {
  usePEffect(() => {
    if (!rackRef.current) return;
    // mapping: [{ moduleSelector: '.mod-atlas', jacks: ['out-x','out-y','out-h','out-grad'] }]
    mapping.forEach(({ selector, jacks }) => {
      const mod = rackRef.current.querySelector(selector);
      if (!mod) return;
      const els = mod.querySelectorAll(".jack");
      jacks.forEach((id, i) => { if (els[i]) els[i].dataset.jack = id; });
    });
  }, [mapping, rackRef]);
}

/* =====================================================
   THE TORNADO PATCH
   ===================================================== */
function TornadoRack() {
  const rackRef = usePRef(null);
  const [step, setStep] = usePState(0);

  usePEffect(() => {
    const id = setInterval(() => setStep(s => (s + 1) % 16), 180);
    return () => clearInterval(id);
  }, []);

  // module ordering (each gets a class for cable lookup)
  useStampJacks(rackRef, [
    { selector: ".mod-atlas",     jacks: ["atlas-x","atlas-y","atlas-h","atlas-grad"] },
    { selector: ".mod-continuum", jacks: ["cont-in-x","cont-in-y","cont-in-f","cont-out-v","cont-out-w","cont-out-p"] },
    { selector: ".mod-instinct",  jacks: ["inst-cv1","inst-cv2","inst-cv3","inst-cv4"] },
    { selector: ".mod-chronicle", jacks: ["chr-clk","chr-rst","chr-ga","chr-gb","chr-cv1","chr-cv2"] },
    { selector: ".mod-composer",  jacks: ["comp-in-v","comp-in-w","comp-l","comp-r"] },
    { selector: ".mod-quill",     jacks: ["quill-arc","quill-beat","quill-loop"] },
    { selector: ".mod-forge",     jacks: ["forge-l","forge-r"] },
  ]);

  const cables = [
    // ATLAS sample → CONTINUUM input
    { from: "atlas-x", to: "cont-in-x", color: "var(--signal-cool)" },
    { from: "atlas-y", to: "cont-in-y", color: "var(--signal-cool)" },
    // INSTINCT modulates CONTINUUM force
    { from: "inst-cv1", to: "cont-in-f", color: "var(--signal-myth)" },
    // CONTINUUM velocity → COMPOSER
    { from: "cont-out-v", to: "comp-in-v", color: "var(--signal-life)" },
    { from: "cont-out-w", to: "comp-in-w", color: "var(--signal-life)" },
    // COMPOSER → FORGE master
    { from: "comp-l", to: "forge-l", color: "var(--signal-warm)" },
    { from: "comp-r", to: "forge-r", color: "var(--signal-warm)" },
    // CHRONICLE clock → QUILL arc
    { from: "chr-cv1", to: "quill-arc", color: "var(--wire-clock)" },
    // CHRONICLE gate → INSTINCT trigger
    { from: "chr-ga", to: "inst-cv4", color: "var(--signal-life)" },
  ];

  return (
    <div className="rack" ref={rackRef}>
      <div className="rack-row">
        <div className="mod-atlas"><ModuleAtlas/></div>
        <div className="mod-continuum"><ModuleContinuum/></div>
        <div className="mod-instinct"><ModuleInstinct/></div>
        <div className="mod-chronicle"><ModuleChronicle step={step}/></div>
        <div className="mod-composer"><ModuleComposer/></div>
        <div className="mod-quill"><ModuleQuill/></div>
        <div className="mod-forge"><ModuleForge/></div>
      </div>
      <PatchCables rackRef={rackRef} cables={cables}/>
    </div>
  );
}

/* =====================================================
   NODE GRAPH (Houdini style)
   ===================================================== */
const NODES = [
  { id:"src",    name:"atlas_terrain",   op:"sample",      x: 40,  y: 60,  ch:"cool",  flag:"L" },
  { id:"noise",  name:"perlin_curl",     op:"noise3d",     x: 200, y: 30,  ch:"life",  flag:"D" },
  { id:"force",  name:"force_field",     op:"vortex",      x: 200, y: 130, ch:"warm",  flag:"" },
  { id:"sim",    name:"continuum_sim",   op:"flip_solver", x: 380, y: 80,  ch:"life",  flag:"R" },
  { id:"behavior",name:"instinct_lfo",   op:"lfo_bank",    x: 380, y: 200, ch:"myth",  flag:"" },
  { id:"merge",  name:"merge",           op:"merge",       x: 540, y: 130, ch:"axiom", flag:"" },
  { id:"voice",  name:"composer_voice",  op:"granulator",  x: 700, y: 90,  ch:"warm",  flag:"D" },
  { id:"narr",   name:"quill_arc",       op:"narrative",   x: 700, y: 200, ch:"myth",  flag:"" },
  { id:"out",    name:"forge_out",       op:"audio_out",   x: 860, y: 145, ch:"forge", flag:"R" },
];
const EDGES = [
  { from:"src", to:"sim", port:0 },
  { from:"noise", to:"sim", port:1 },
  { from:"force", to:"sim", port:2 },
  { from:"sim", to:"merge", port:0 },
  { from:"behavior", to:"merge", port:1 },
  { from:"merge", to:"voice", port:0 },
  { from:"merge", to:"narr", port:0 },
  { from:"voice", to:"out", port:0 },
  { from:"narr",  to:"out", port:1 },
];

function NodeGraph() {
  const w = 980, h = 280;
  const byId = usePMemo(() => Object.fromEntries(NODES.map(n => [n.id, n])), []);
  return (
    <div className="ng mat-cavity">
      <svg className="ng-svg" width={w} height={h} viewBox={`0 0 ${w} ${h}`}>
        {/* dot grid */}
        <defs>
          <pattern id="ng-grid" width="20" height="20" patternUnits="userSpaceOnUse">
            <circle cx="10" cy="10" r="0.7" fill="rgba(255,255,255,0.06)"/>
          </pattern>
        </defs>
        <rect width={w} height={h} fill="url(#ng-grid)"/>

        {/* edges */}
        {EDGES.map((e, i) => {
          const a = byId[e.from], b = byId[e.to];
          const x1 = a.x + 110, y1 = a.y + 22;
          const x2 = b.x,       y2 = b.y + 22;
          const cx = (x1 + x2) / 2;
          return (
            <g key={i}>
              <path d={`M ${x1} ${y1} C ${cx} ${y1}, ${cx} ${y2}, ${x2} ${y2}`}
                    stroke={`var(--signal-${a.ch})`} strokeWidth="1.5" fill="none" opacity="0.85"
                    style={{ filter: `drop-shadow(0 0 4px var(--signal-${a.ch}))` }}/>
              {/* output port circle */}
              <circle cx={x1} cy={y1} r="3" fill={`var(--signal-${a.ch})`}
                      style={{ filter: `drop-shadow(0 0 3px var(--signal-${a.ch}))` }}/>
              <circle cx={x2} cy={y2} r="3" fill={`var(--signal-${a.ch})`}
                      style={{ filter: `drop-shadow(0 0 3px var(--signal-${a.ch}))` }}/>
            </g>
          );
        })}

        {/* nodes */}
        {NODES.map(n => (
          <g key={n.id} transform={`translate(${n.x} ${n.y})`}>
            {/* body */}
            <rect width="110" height="44" rx="3"
                  fill="#14181f"
                  stroke={`var(--signal-${n.ch})`}
                  strokeWidth="1.2"
                  style={{ filter: `drop-shadow(0 0 4px var(--signal-${n.ch}))` }}/>
            {/* header strip */}
            <rect width="110" height="14" rx="3" fill={`var(--signal-${n.ch})`} opacity="0.18"/>
            <rect y="12" width="110" height="2" fill={`var(--signal-${n.ch})`} opacity="0.6"/>
            {/* op text */}
            <text x="6" y="10" fill={`var(--signal-${n.ch})`} fontSize="8" fontFamily="var(--font-mono)" letterSpacing="0.1em">
              {n.op.toUpperCase()}
            </text>
            {/* name */}
            <text x="6" y="26" fill="#e6ebf2" fontSize="10" fontFamily="var(--font-mono)">
              {n.name}
            </text>
            {/* flags (Houdini style: D=display, R=render, L=lock) */}
            <g transform="translate(86, 30)">
              {[..."DRL"].map((c, i) => {
                const on = n.flag.includes(c);
                return (
                  <g key={c} transform={`translate(${i*8} 0)`}>
                    <circle r="3" fill={on ? `var(--signal-${n.ch})` : "rgba(255,255,255,0.08)"}
                            style={on ? { filter: `drop-shadow(0 0 3px var(--signal-${n.ch}))` } : {}}/>
                    <text x="0" y="2" fill={on ? "#000" : "rgba(255,255,255,0.3)"} fontSize="5" textAnchor="middle" fontFamily="var(--font-mono)">{c}</text>
                  </g>
                );
              })}
            </g>
            {/* input ports (dots on left) */}
            <circle cx="0" cy="22" r="2.5" fill="#0a0c10" stroke={`var(--signal-${n.ch})`} strokeWidth="1"/>
            <circle cx="110" cy="22" r="2.5" fill="#0a0c10" stroke={`var(--signal-${n.ch})`} strokeWidth="1"/>
          </g>
        ))}

        {/* viewport breadcrumb */}
        <g transform={`translate(${w-180}, 12)`}>
          <rect width="170" height="20" rx="2" fill="rgba(0,0,0,0.5)" stroke="rgba(255,255,255,0.06)"/>
          <text x="8" y="13" fontSize="9" fontFamily="var(--font-mono)" fill="var(--ink-mid)" letterSpacing="0.08em">
            /obj/tornado/sim_v07
          </text>
        </g>
      </svg>
    </div>
  );
}

/* =====================================================
   TIMELINE (Maya style)
   ===================================================== */
function Timeline() {
  const tracks = [
    { name:"atlas.position",  ch:"cool",  keys:[6, 24, 48, 72, 96] },
    { name:"continuum.viscosity", ch:"life", keys:[12, 36, 84] },
    { name:"instinct.envelope.attack", ch:"myth", keys:[0, 24, 48, 72, 96, 120] },
    { name:"chronicle.bpm", ch:"amber", keys:[0, 60] },
    { name:"composer.pitch", ch:"warm", keys:[18, 42, 60, 90, 110] },
    { name:"quill.tension", ch:"rose", keys:[0, 48, 96] },
    { name:"forge.master", ch:"forge", keys:[0, 120] },
  ];
  const FRAMES = 120;
  const pxPer = 7;
  const [playhead, setPlayhead] = usePState(34);
  usePEffect(() => {
    const id = setInterval(() => setPlayhead(p => (p + 1) % (FRAMES + 1)), 80);
    return () => clearInterval(id);
  }, []);

  return (
    <div className="tl mat-cavity">
      {/* header */}
      <div className="tl-head">
        <div className="tl-controls">
          <Pad label="◀◀" channel="cool" size={26}/>
          <Pad label="◀"  channel="cool" size={26}/>
          <Pad label="▶"  channel="life" size={26} lit/>
          <Pad label="■"  channel="hot"  size={26}/>
          <Pad label="●"  channel="hot"  size={26}/>
          <div className="tl-readouts">
            <Readout label="FRAME" value={String(playhead).padStart(3,"0")} channel="cool" width={64}/>
            <Readout label="TIME"  value={`${(playhead/24).toFixed(2)}s`} channel="cool" width={64}/>
            <Readout label="FPS"   value="24.0" channel="amber" width={50}/>
          </div>
        </div>
        {/* ruler */}
        <div className="tl-ruler" style={{ width: FRAMES * pxPer }}>
          {Array.from({ length: FRAMES + 1 }).map((_, i) => (
            <div key={i} className={`tl-tick ${i % 12 === 0 ? "major" : i % 6 === 0 ? "mid":""}`}>
              {i % 12 === 0 && <span>{i}</span>}
            </div>
          ))}
        </div>
      </div>
      {/* tracks */}
      <div className="tl-body">
        {tracks.map(t => (
          <div key={t.name} className="tl-track">
            <div className="tl-track-name">
              <span className="tl-dot" style={{background: `var(--signal-${t.ch})`, boxShadow: `0 0 4px var(--signal-${t.ch})`}}/>
              {t.name}
            </div>
            <div className="tl-lane" style={{ width: FRAMES * pxPer }}>
              <div className="tl-lane-bg"/>
              {/* curve mini */}
              <svg className="tl-curve" width={FRAMES * pxPer} height="22" viewBox={`0 0 ${FRAMES*pxPer} 22`}>
                <path d={t.keys.map((k,i) => {
                  const x = k * pxPer;
                  const y = 4 + ((Math.sin(i*1.3 + k*0.05) + 1)/2) * 14;
                  return (i===0 ? `M ${x} ${y}` : `L ${x} ${y}`);
                }).join(" ")} fill="none" stroke={`var(--signal-${t.ch})`} strokeWidth="1.2" opacity="0.8"
                  style={{ filter: `drop-shadow(0 0 3px var(--signal-${t.ch}))` }}/>
              </svg>
              {/* keyframes */}
              {t.keys.map(k => (
                <div key={k} className="tl-key" style={{
                  left: k * pxPer - 4,
                  background: `var(--signal-${t.ch})`,
                  boxShadow: `0 0 6px var(--signal-${t.ch})`,
                }}/>
              ))}
            </div>
          </div>
        ))}
        {/* playhead */}
        <div className="tl-playhead" style={{ left: 200 + playhead * pxPer }}>
          <div className="tl-playhead-handle"/>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { TornadoRack, NodeGraph, Timeline });
