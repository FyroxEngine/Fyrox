/* Quantum Atlas — Map Canvas Component
   Renders a 2D bioluminescent world map with animated nodes,
   region highlights, and a 3D globe toggle.
   Depends on: React (global), quill-os.css tokens
*/

const { useState, useEffect, useRef, useCallback } = React;

// ── World data ─────────────────────────────────────────────────────
const REGIONS = [
  { id: 'vyr', name: 'Vyreth Expanse',    type: 'void-zone',   color: '#00e5ff', glow: 'rgba(0,229,255,0.5)',    x: 22,  y: 28,  rx: 11, ry: 8,  hz: 432, stratum: 3, lore: 'A vast quantum expanse where probability fields collapse into landmass. No two surveys have mapped it the same.' },
  { id: 'sel', name: 'Selenarch Drift',   type: 'arcane-node', color: '#c084fc', glow: 'rgba(192,132,252,0.5)', x: 48,  y: 20,  rx: 8,  ry: 6,  hz: 741, stratum: 7, lore: 'The Selenarch Drift is a mythos-saturated corridor where narrative threads become physical geography.' },
  { id: 'bri', name: 'Brine Lattice',     type: 'bio-zone',    color: '#39ff14', glow: 'rgba(57,255,20,0.5)',   x: 70,  y: 35,  rx: 10, ry: 7,  hz: 396, stratum: 2, lore: 'Living crystalline formations that grow, die, and regrow on 6-hour cycles. Classified: BIO-2.' },
  { id: 'aur', name: 'Auric Threshold',   type: 'forge-node',  color: '#fbbf24', glow: 'rgba(251,191,36,0.5)',  x: 35,  y: 55,  rx: 9,  ry: 7,  hz: 528, stratum: 5, lore: 'A forge-resonant zone where gold-frequency matter accumulates. The Order claims it as sovereign territory.' },
  { id: 'nox', name: 'Nox Basin',         type: 'void-zone',   color: '#00e5ff', glow: 'rgba(0,229,255,0.5)',   x: 60,  y: 62,  rx: 7,  ry: 5,  hz: 285, stratum: 1, lore: 'Deepest mapped stratum. Instruments lose coherence below 200hz here. Entry requires Prime seal.' },
  { id: 'emb', name: 'Ember Reach',       type: 'plasma-zone', color: '#f97316', glow: 'rgba(249,115,22,0.5)',  x: 82,  y: 55,  rx: 6,  ry: 8,  hz: 639, stratum: 4, lore: 'A plasma-temperature zone at the eastern terminus. Cartography requires thermal-shielded probes.' },
  { id: 'syl', name: 'Sylvan Meridian',   type: 'bio-zone',    color: '#39ff14', glow: 'rgba(57,255,20,0.5)',   x: 15,  y: 65,  rx: 8,  ry: 6,  hz: 432, stratum: 2, lore: 'Dense bioluminescent forest-analog. Organic data structures observed; classification pending.' },
  { id: 'prs', name: 'Prime Seat',        type: 'prime-node',  color: '#fbbf24', glow: 'rgba(251,191,36,0.8)',  x: 50,  y: 42,  rx: 4,  ry: 4,  hz: 432, stratum: 0, lore: 'The Prime Seat. Origin of all Lineage threads. The Forge is lit here, always.' },
];

const ROUTES = [
  { from: 'prs', to: 'vyr', color: '#00e5ff', opacity: 0.4 },
  { from: 'prs', to: 'sel', color: '#c084fc', opacity: 0.35 },
  { from: 'prs', to: 'aur', color: '#fbbf24', opacity: 0.45 },
  { from: 'prs', to: 'bri', color: '#39ff14', opacity: 0.3 },
  { from: 'prs', to: 'nox', color: '#00e5ff', opacity: 0.25 },
  { from: 'aur', to: 'emb', color: '#f97316', opacity: 0.3 },
  { from: 'vyr', to: 'syl', color: '#39ff14', opacity: 0.2 },
  { from: 'sel', to: 'bri', color: '#c084fc', opacity: 0.2 },
];

const SURVEY_POINTS = [
  { id: 'sp1', x: 30, y: 18, color: '#00e5ff', label: 'QSV·001' },
  { id: 'sp2', x: 55, y: 70, color: '#39ff14', label: 'BSV·014' },
  { id: 'sp3', x: 78, y: 22, color: '#c084fc', label: 'MSV·007' },
  { id: 'sp4', x: 88, y: 72, color: '#f97316', label: 'ESV·003' },
  { id: 'sp5', x: 8,  y: 45, color: '#fbbf24', label: 'GSV·011' },
];

function getRegionById(id) { return REGIONS.find(r => r.id === id); }
function getXY(id, w, h) {
  const r = getRegionById(id);
  return r ? { x: (r.x / 100) * w, y: (r.y / 100) * h } : null;
}

// ── 2D Map Canvas ──────────────────────────────────────────────────
function MapCanvas2D({ selectedId, onSelect, layers, mapTone, zoom, pan, regions: propRegions, routes: propRoutes, survey: propSurvey }) {
  // Use props if provided, fall back to module-level defaults
  const REGIONS_DATA = propRegions || REGIONS;
  const ROUTES_DATA  = propRoutes  || ROUTES;
  const SURVEY_DATA  = propSurvey  || SURVEY_POINTS;
  const svgRef = useRef(null);
  const [t, setT] = useState(0);
  const [hoverId, setHoverId] = useState(null);
  const animRef = useRef(null);

  useEffect(() => {
    let frame;
    const tick = () => { setT(x => x + 1); frame = requestAnimationFrame(tick); };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, []);

  const W = 1000, H = 660;

  // Starfield
  const stars = useRef(Array.from({ length: 120 }, (_, i) => ({
    x: Math.random() * 100, y: Math.random() * 100,
    r: Math.random() * 0.6 + 0.2, op: Math.random() * 0.5 + 0.15,
    phase: Math.random() * Math.PI * 2,
  }))).current;

  // Contour lines (procedural terrain)
  const contours = useRef(Array.from({ length: 18 }, (_, i) => {
    const cx = 30 + Math.random() * 40, cy = 20 + Math.random() * 60;
    const rx = 4 + Math.random() * 12, ry = 3 + Math.random() * 8;
    return { cx, cy, rx, ry, color: Math.random() > 0.6 ? '#00e5ff' : Math.random() > 0.5 ? '#c084fc' : '#fbbf24', op: 0.04 + Math.random() * 0.06 };
  })).current;

  const activeRegions = REGIONS_DATA.filter(r => {
    if (r.type === 'void-zone'   && !layers.void)   return false;
    if (r.type === 'arcane-node' && !layers.arcane) return false;
    if (r.type === 'bio-zone'    && !layers.bio)    return false;
    if (r.type === 'forge-node'  && !layers.forge)  return false;
    if (r.type === 'plasma-zone' && !layers.plasma) return false;
    if (r.type === 'prime-node'  && !layers.prime)  return false;
    return true;
  });

  return (
    <svg
      ref={svgRef}
      viewBox={`0 0 ${W} ${H}`}
      style={{ width: '100%', height: '100%', display: 'block', cursor: 'crosshair' }}
      xmlns="http://www.w3.org/2000/svg"
    >
      <defs>
        {/* Void background */}
        <radialGradient id="bgGrad" cx="50%" cy="45%" r="70%">
          <stop offset="0%"   stopColor="#0a1428" />
          <stop offset="50%"  stopColor="#060c1a" />
          <stop offset="100%" stopColor="#03050a" />
        </radialGradient>
        {/* Nebula clouds */}
        <radialGradient id="neb1" cx="20%" cy="30%" r="45%">
          <stop offset="0%" stopColor="rgba(0,100,180,0.18)"/>
          <stop offset="100%" stopColor="transparent"/>
        </radialGradient>
        <radialGradient id="neb2" cx="75%" cy="65%" r="40%">
          <stop offset="0%" stopColor="rgba(80,20,120,0.15)"/>
          <stop offset="100%" stopColor="transparent"/>
        </radialGradient>
        <radialGradient id="neb3" cx="50%" cy="80%" r="35%">
          <stop offset="0%" stopColor="rgba(0,60,100,0.12)"/>
          <stop offset="100%" stopColor="transparent"/>
        </radialGradient>
        {/* Region glows */}
        {REGIONS_DATA.map(r => (
          <radialGradient key={`rg-${r.id}`} id={`rg-${r.id}`} cx="50%" cy="50%" r="50%">
            <stop offset="0%" stopColor={r.color} stopOpacity="0.35"/>
            <stop offset="60%" stopColor={r.color} stopOpacity="0.08"/>
            <stop offset="100%" stopColor={r.color} stopOpacity="0"/>
          </radialGradient>
        ))}
        <filter id="glow-strong">
          <feGaussianBlur stdDeviation="3" result="blur"/>
          <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
        </filter>
        <filter id="glow-soft">
          <feGaussianBlur stdDeviation="6" result="blur"/>
          <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
        </filter>
        <filter id="glow-pulse">
          <feGaussianBlur stdDeviation="2" result="blur"/>
          <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
        </filter>
        <clipPath id="mapClip">
          <rect x="0" y="0" width={W} height={H}/>
        </clipPath>
      </defs>

      {/* Background */}
      <rect width={W} height={H} fill="url(#bgGrad)"/>
      <rect width={W} height={H} fill="url(#neb1)"/>
      <rect width={W} height={H} fill="url(#neb2)"/>
      <rect width={W} height={H} fill="url(#neb3)"/>

      {/* Starfield */}
      {stars.map((s, i) => (
        <circle key={i}
          cx={s.x * W / 100} cy={s.y * H / 100} r={s.r}
          fill="white"
          opacity={s.op * (0.7 + 0.3 * Math.sin(t * 0.02 + s.phase))}
        />
      ))}

      {/* Contour terrain lines */}
      {contours.map((c, i) => (
        <ellipse key={i}
          cx={c.cx * W / 100} cy={c.cy * H / 100}
          rx={c.rx * W / 100} ry={c.ry * H / 100}
          fill="none" stroke={c.color} strokeWidth="0.5" opacity={c.op}
        />
      ))}

      {/* Hex grid overlay */}
      {layers.grid && Array.from({ length: 20 }, (_, row) =>
        Array.from({ length: 14 }, (_, col) => {
          const hx = (col * 5.5 + (row % 2) * 2.75) * W / 100;
          const hy = (row * 4.8) * H / 100;
          const pts = Array.from({length:6},(_,i)=>{
            const a = (Math.PI/3)*i - Math.PI/6;
            return `${hx+Math.cos(a)*22},${hy+Math.sin(a)*22}`;
          }).join(' ');
          return <polygon key={`${row}-${col}`} points={pts}
            fill="none" stroke="rgba(0,229,255,0.04)" strokeWidth="0.4"/>;
        })
      )}

      {/* Routes / threads */}
      {layers.routes && ROUTES_DATA.map((rt, i) => {
        const getRegDyn = (id) => REGIONS_DATA.find(r => r.id === id);
        const getXYDyn  = (id) => { const r = getRegDyn(id); return r ? { x: (r.x/100)*W, y: (r.y/100)*H } : null; };
        const a = getXYDyn(rt.from, W, H), b = getXYDyn(rt.to, W, H);
        if (!a || !b) return null;
        const mx = (a.x + b.x) / 2, my = (a.y + b.y) / 2 - 40;
        const dashOffset = -(t * 0.8) % 20;
        return (
          <g key={i}>
            <path d={`M${a.x},${a.y} Q${mx},${my} ${b.x},${b.y}`}
              fill="none" stroke={rt.color} strokeWidth="0.8"
              strokeOpacity={rt.opacity} strokeDasharray="4 6"
              strokeDashoffset={dashOffset}
            />
            <path d={`M${a.x},${a.y} Q${mx},${my} ${b.x},${b.y}`}
              fill="none" stroke={rt.color} strokeWidth="2"
              strokeOpacity={rt.opacity * 0.3}
            />
          </g>
        );
      })}

      {/* Regions */}
      {activeRegions.map(r => {
        const cx = (r.x / 100) * W, cy = (r.y / 100) * H;
        const rx = (r.rx / 100) * W, ry = (r.ry / 100) * H;
        const isSelected = selectedId === r.id;
        const isHovered  = hoverId === r.id;
        const pulse = 0.5 + 0.5 * Math.sin(t * 0.04 + r.hz * 0.001);
        const scale = isSelected ? 1.08 : isHovered ? 1.04 : 1.0;
        const isPrime = r.type === 'prime-node';

        return (
          <g key={r.id}
            style={{ cursor: 'pointer', transition: 'all 200ms' }}
            transform={`translate(${cx},${cy}) scale(${scale}) translate(${-cx},${-cy})`}
            onClick={() => onSelect(r.id)}
            onMouseEnter={() => setHoverId(r.id)}
            onMouseLeave={() => setHoverId(null)}
          >
            {/* Outer glow */}
            <ellipse cx={cx} cy={cy} rx={rx * 2.2} ry={ry * 2.2}
              fill={`url(#rg-${r.id})`} opacity={0.4 + 0.2 * pulse}/>

            {/* Region fill */}
            <ellipse cx={cx} cy={cy} rx={rx} ry={ry}
              fill={r.color} fillOpacity={isSelected ? 0.22 : 0.10}
              stroke={r.color} strokeWidth={isSelected ? 1.8 : 1}
              strokeOpacity={isSelected ? 1 : 0.6}
              filter={isSelected || isHovered ? 'url(#glow-strong)' : undefined}
            />

            {/* Prime extra ring */}
            {isPrime && (
              <>
                <circle cx={cx} cy={cy} r={rx * 2.5}
                  fill="none" stroke={r.color} strokeWidth="0.6"
                  strokeOpacity="0.3" strokeDasharray="3 4"
                  style={{transformOrigin:`${cx}px ${cy}px`, animation:'spin-slow 20s linear infinite'}}
                />
                <circle cx={cx} cy={cy} r={rx * 1.8}
                  fill="none" stroke={r.color} strokeWidth="0.8"
                  strokeOpacity={0.4 + 0.3 * pulse}
                />
              </>
            )}

            {/* Node dot */}
            <circle cx={cx} cy={cy} r={isPrime ? 6 : 4}
              fill={r.color} filter="url(#glow-pulse)"
              opacity={0.8 + 0.2 * pulse}
            />

            {/* Pulse ring */}
            <circle cx={cx} cy={cy} r={(isPrime ? 10 : 7) + pulse * 4}
              fill="none" stroke={r.color} strokeWidth="0.7"
              opacity={(1 - pulse) * 0.7}
            />

            {/* Label */}
            {(isSelected || isHovered || isPrime) && (
              <g>
                <rect x={cx - 38} y={cy - ry - 20} width="76" height="14"
                  rx="1" fill="rgba(3,5,10,0.82)" stroke={r.color} strokeWidth="0.5" strokeOpacity="0.5"/>
                <text x={cx} y={cy - ry - 10}
                  textAnchor="middle" fontFamily="Cinzel, serif"
                  fontSize="7" fill={r.color} letterSpacing="1.5">
                  {r.name.toUpperCase()}
                </text>
              </g>
            )}

            {/* Type chip */}
            <text x={cx + rx + 4} y={cy + 3}
              fontFamily="JetBrains Mono, monospace" fontSize="6"
              fill={r.color} opacity="0.55" letterSpacing="0.5">
              {r.hz}HZ
            </text>
          </g>
        );
      })}

      {/* Survey points */}
      {layers.survey && SURVEY_DATA.map(sp => {
        const sx = (sp.x / 100) * W, sy = (sp.y / 100) * H;
        const pulse = 0.5 + 0.5 * Math.sin(t * 0.05 + sx * 0.01);
        return (
          <g key={sp.id} style={{ cursor: 'pointer' }}>
            <line x1={sx - 5} y1={sy} x2={sx + 5} y2={sy} stroke={sp.color} strokeWidth="1" opacity="0.7"/>
            <line x1={sx} y1={sy - 5} x2={sx} y2={sy + 5} stroke={sp.color} strokeWidth="1" opacity="0.7"/>
            <circle cx={sx} cy={sy} r={3 + pulse * 2}
              fill="none" stroke={sp.color} strokeWidth="0.7" opacity={(1 - pulse) * 0.6}/>
            <circle cx={sx} cy={sy} r="2" fill={sp.color} opacity="0.9" filter="url(#glow-pulse)"/>
            <text x={sx + 5} y={sy - 5}
              fontFamily="JetBrains Mono, monospace" fontSize="5.5"
              fill={sp.color} opacity="0.65" letterSpacing="0.5">{sp.label}</text>
          </g>
        );
      })}

      {/* Coordinate grid overlay */}
      {layers.grid && (
        <g opacity="0.12">
          {Array.from({length:11}, (_,i) => (
            <line key={`v${i}`} x1={i*(W/10)} y1={0} x2={i*(W/10)} y2={H}
              stroke="#00e5ff" strokeWidth="0.3"/>
          ))}
          {Array.from({length:8}, (_,i) => (
            <line key={`h${i}`} x1={0} y1={i*(H/7)} x2={W} y2={i*(H/7)}
              stroke="#00e5ff" strokeWidth="0.3"/>
          ))}
        </g>
      )}

      {/* Compass rose */}
      <g transform={`translate(${W - 60}, 50)`} opacity="0.55">
        <circle cx="0" cy="0" r="22" fill="none" stroke="#fbbf24" strokeWidth="0.7" strokeDasharray="2 3"/>
        <circle cx="0" cy="0" r="14" fill="none" stroke="#fbbf24" strokeWidth="0.5" opacity="0.5"/>
        <polygon points="0,-18 3,-4 0,-7 -3,-4" fill="#fbbf24"/>
        <polygon points="0,18 3,4 0,7 -3,4" fill="#fbbf24" opacity="0.5"/>
        <polygon points="-18,0 -4,-3 -7,0 -4,3" fill="#fbbf24" opacity="0.5"/>
        <polygon points="18,0 4,-3 7,0 4,3" fill="#fbbf24" opacity="0.5"/>
        <text x="0" y="-22" textAnchor="middle" fontFamily="Cinzel, serif"
          fontSize="6" fill="#fbbf24" letterSpacing="1">N</text>
      </g>

      {/* Scale bar */}
      <g transform={`translate(40, ${H - 30})`} opacity="0.5">
        <line x1="0" y1="0" x2="80" y2="0" stroke="#94a3b8" strokeWidth="1"/>
        <line x1="0" y1="-4" x2="0" y2="4" stroke="#94a3b8" strokeWidth="1"/>
        <line x1="80" y1="-4" x2="80" y2="4" stroke="#94a3b8" strokeWidth="1"/>
        <text x="40" y="-7" textAnchor="middle" fontFamily="JetBrains Mono, monospace"
          fontSize="7" fill="#94a3b8" letterSpacing="1">500 QU</text>
      </g>

      {/* Coordinate readout */}
      <text x={W - 12} y={H - 10}
        textAnchor="end" fontFamily="JetBrains Mono, monospace"
        fontSize="7" fill="#475569" letterSpacing="0.5">
        STRATUM · PRIME · QUANTUM ATLAS v0.9.3
      </text>
    </svg>
  );
}

// ── 3D Globe Canvas ────────────────────────────────────────────────
function MapCanvas3D({ selectedId, onSelect }) {
  const canvasRef = useRef(null);
  const tRef = useRef(0);
  const animRef = useRef(null);
  const [hoverId, setHoverId] = useState(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    const W = canvas.width, H = canvas.height;
    const CX = W / 2, CY = H / 2;
    const R = Math.min(W, H) * 0.36;

    function drawFrame() {
      const t = tRef.current;
      tRef.current += 0.008;

      ctx.clearRect(0, 0, W, H);

      // Deep void bg
      const bg = ctx.createRadialGradient(CX, CY, 0, CX, CY, Math.max(W, H) * 0.7);
      bg.addColorStop(0, '#0a1428');
      bg.addColorStop(0.5, '#060c1a');
      bg.addColorStop(1, '#03050a');
      ctx.fillStyle = bg;
      ctx.fillRect(0, 0, W, H);

      // Stars
      for (let i = 0; i < 200; i++) {
        const sx = ((i * 137 + 41) % W);
        const sy = ((i * 97 + 17) % H);
        const sr = 0.4 + (i % 3) * 0.3;
        const op = 0.2 + 0.3 * Math.sin(t * 0.5 + i * 0.8);
        ctx.beginPath();
        ctx.arc(sx, sy, sr, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(255,255,255,${op})`;
        ctx.fill();
      }

      // Globe shadow halo
      const halo = ctx.createRadialGradient(CX, CY, R * 0.85, CX, CY, R * 1.5);
      halo.addColorStop(0, 'rgba(0,100,200,0.12)');
      halo.addColorStop(0.5, 'rgba(80,0,160,0.08)');
      halo.addColorStop(1, 'transparent');
      ctx.fillStyle = halo;
      ctx.beginPath();
      ctx.arc(CX, CY, R * 1.5, 0, Math.PI * 2);
      ctx.fill();

      // Globe base
      const globeGrad = ctx.createRadialGradient(CX - R * 0.25, CY - R * 0.25, R * 0.05, CX, CY, R);
      globeGrad.addColorStop(0, '#1a2a4a');
      globeGrad.addColorStop(0.4, '#0d1a30');
      globeGrad.addColorStop(0.85, '#060c1a');
      globeGrad.addColorStop(1, '#03050a');
      ctx.beginPath();
      ctx.arc(CX, CY, R, 0, Math.PI * 2);
      ctx.fillStyle = globeGrad;
      ctx.fill();

      // Latitude lines
      for (let lat = -75; lat <= 75; lat += 15) {
        const latR = Math.abs(Math.cos(lat * Math.PI / 180)) * R;
        const latY = CY + Math.sin(lat * Math.PI / 180) * R;
        if (latR < 2) continue;
        ctx.beginPath();
        ctx.ellipse(CX, latY, latR, latR * 0.08, 0, 0, Math.PI * 2);
        ctx.strokeStyle = 'rgba(0,180,255,0.12)';
        ctx.lineWidth = 0.5;
        ctx.stroke();
      }

      // Longitude lines
      for (let lon = 0; lon < 180; lon += 20) {
        const angle = (lon * Math.PI / 180) + t;
        const cosA = Math.cos(angle);
        ctx.beginPath();
        ctx.ellipse(CX, CY, Math.abs(cosA) * R, R, 0, 0, Math.PI * 2);
        ctx.strokeStyle = `rgba(0,180,255,${0.05 + (cosA > 0 ? 0.05 : 0)})`;
        ctx.lineWidth = 0.5;
        ctx.stroke();
      }

      // Continent blobs (procedural)
      const continents = [
        { lon: 0.4, lat: 0.2, w: 0.28, h: 0.18, color: '#00e5ff', op: 0.35 },
        { lon: 1.8, lat: -0.1, w: 0.22, h: 0.14, color: '#c084fc', op: 0.3 },
        { lon: 3.2, lat: 0.3, w: 0.18, h: 0.12, color: '#39ff14', op: 0.3 },
        { lon: 2.4, lat: -0.35, w: 0.15, h: 0.1,  color: '#fbbf24', op: 0.35 },
        { lon: 0.9, lat: -0.25, w: 0.12, h: 0.09, color: '#f97316', op: 0.3 },
      ];
      continents.forEach(c => {
        const lonAngle = c.lon + t;
        const cosLon = Math.cos(lonAngle);
        if (cosLon < 0) return; // back-face cull
        const screenX = CX + Math.sin(lonAngle) * R * 0.9;
        const screenY = CY + c.lat * R;
        const scaleX = Math.abs(cosLon);
        ctx.save();
        ctx.translate(screenX, screenY);
        ctx.scale(scaleX, 1);
        const g = ctx.createRadialGradient(0, 0, 0, 0, 0, c.w * R);
        g.addColorStop(0, c.color.replace(')', `,${c.op})`).replace('rgb', 'rgba').replace('#', 'rgba(') );
        g.addColorStop(1, 'transparent');
        // Simple radial fill workaround for hex colors
        ctx.beginPath();
        ctx.ellipse(0, 0, c.w * R, c.h * R, 0, 0, Math.PI * 2);
        ctx.fillStyle = c.color + '55';
        ctx.fill();
        ctx.strokeStyle = c.color + '99';
        ctx.lineWidth = 1;
        ctx.stroke();
        ctx.restore();
      });

      // Orbit ring
      ctx.save();
      ctx.beginPath();
      ctx.ellipse(CX, CY, R * 1.35, R * 0.18, 0, 0, Math.PI * 2);
      ctx.strokeStyle = 'rgba(251,191,36,0.25)';
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 6]);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.restore();

      // Orbiting satellite dot
      const satAngle = t * 1.5;
      const satX = CX + Math.cos(satAngle) * R * 1.35;
      const satY = CY + Math.sin(satAngle) * R * 0.18;
      ctx.beginPath();
      ctx.arc(satX, satY, 3, 0, Math.PI * 2);
      ctx.fillStyle = '#fbbf24';
      ctx.shadowBlur = 8;
      ctx.shadowColor = '#fbbf24';
      ctx.fill();
      ctx.shadowBlur = 0;

      // Globe highlight (specular)
      const spec = ctx.createRadialGradient(CX - R * 0.35, CY - R * 0.35, 0, CX - R * 0.2, CY - R * 0.2, R * 0.6);
      spec.addColorStop(0, 'rgba(160,200,255,0.09)');
      spec.addColorStop(1, 'transparent');
      ctx.beginPath();
      ctx.arc(CX, CY, R, 0, Math.PI * 2);
      ctx.fillStyle = spec;
      ctx.fill();

      // Atmosphere rim
      ctx.beginPath();
      ctx.arc(CX, CY, R, 0, Math.PI * 2);
      ctx.strokeStyle = 'rgba(0,180,255,0.35)';
      ctx.lineWidth = 2;
      ctx.stroke();

      // Globe label
      ctx.font = '700 11px Cinzel, serif';
      ctx.fillStyle = 'rgba(251,191,36,0.6)';
      ctx.letterSpacing = '3px';
      ctx.textAlign = 'center';
      ctx.fillText('QUANTUM ATLAS · STRATUM PRIME', CX, CY + R + 24);
      ctx.font = '9px JetBrains Mono, monospace';
      ctx.fillStyle = 'rgba(100,116,139,0.7)';
      ctx.fillText(`ROT · ${(t * 180 / Math.PI % 360).toFixed(1)}° · 3D · LIVE`, CX, CY + R + 38);

      animRef.current = requestAnimationFrame(drawFrame);
    }

    animRef.current = requestAnimationFrame(drawFrame);
    return () => cancelAnimationFrame(animRef.current);
  }, []);

  return (
    <canvas
      ref={canvasRef}
      width={1000}
      height={660}
      style={{ width: '100%', height: '100%', display: 'block' }}
    />
  );
}

// ── Main Map View (2D / 3D toggled) ───────────────────────────────
function AtlasMapView({ selectedId, onSelect, layers, view3D, zoom, pan, regions, routes, survey }) {
  return view3D
    ? <MapCanvas3D selectedId={selectedId} onSelect={onSelect} />
    : <MapCanvas2D selectedId={selectedId} onSelect={onSelect} layers={layers} zoom={zoom} pan={pan}
        regions={regions} routes={routes} survey={survey} />;
}

// Export
Object.assign(window, { AtlasMapView, REGIONS, SURVEY_POINTS });
// Keep global alias for legacy panel lookups
window.REGIONS = REGIONS;
