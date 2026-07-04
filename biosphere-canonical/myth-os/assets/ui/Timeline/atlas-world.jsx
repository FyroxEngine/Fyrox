/* Quantum Atlas — World State Store
   Single source of truth for all map entities.
   Exported to window so map + forge + panels all share it.
*/

// ── Default world data ─────────────────────────────────────────────
const DEFAULT_REGIONS = [
  { id: 'vyr', name: 'Vyreth Expanse',   type: 'void-zone',   color: '#00e5ff', glow: 'rgba(0,229,255,0.5)',   x: 22, y: 28, rx: 11, ry: 8,  hz: 432, stratum: 3, lore: 'A vast quantum expanse where probability fields collapse into landmass. No two surveys have mapped it the same.' },
  { id: 'sel', name: 'Selenarch Drift',  type: 'arcane-node', color: '#c084fc', glow: 'rgba(192,132,252,0.5)', x: 48, y: 20, rx: 8,  ry: 6,  hz: 741, stratum: 7, lore: 'The Selenarch Drift is a mythos-saturated corridor where narrative threads become physical geography.' },
  { id: 'bri', name: 'Brine Lattice',    type: 'bio-zone',    color: '#39ff14', glow: 'rgba(57,255,20,0.5)',   x: 70, y: 35, rx: 10, ry: 7,  hz: 396, stratum: 2, lore: 'Living crystalline formations that grow, die, and regrow on 6-hour cycles. Classified: BIO-2.' },
  { id: 'aur', name: 'Auric Threshold',  type: 'forge-node',  color: '#fbbf24', glow: 'rgba(251,191,36,0.5)',  x: 35, y: 55, rx: 9,  ry: 7,  hz: 528, stratum: 5, lore: 'A forge-resonant zone where gold-frequency matter accumulates. The Order claims it as sovereign territory.' },
  { id: 'nox', name: 'Nox Basin',        type: 'void-zone',   color: '#00e5ff', glow: 'rgba(0,229,255,0.5)',   x: 60, y: 62, rx: 7,  ry: 5,  hz: 285, stratum: 1, lore: 'Deepest mapped stratum. Instruments lose coherence below 200hz here. Entry requires Prime seal.' },
  { id: 'emb', name: 'Ember Reach',      type: 'plasma-zone', color: '#f97316', glow: 'rgba(249,115,22,0.5)',  x: 82, y: 55, rx: 6,  ry: 8,  hz: 639, stratum: 4, lore: 'A plasma-temperature zone at the eastern terminus. Cartography requires thermal-shielded probes.' },
  { id: 'syl', name: 'Sylvan Meridian',  type: 'bio-zone',    color: '#39ff14', glow: 'rgba(57,255,20,0.5)',   x: 15, y: 65, rx: 8,  ry: 6,  hz: 432, stratum: 2, lore: 'Dense bioluminescent forest-analog. Organic data structures observed; classification pending.' },
  { id: 'prs', name: 'Prime Seat',       type: 'prime-node',  color: '#fbbf24', glow: 'rgba(251,191,36,0.8)',  x: 50, y: 42, rx: 4,  ry: 4,  hz: 432, stratum: 0, lore: 'The Prime Seat. Origin of all Lineage threads. The Forge is lit here, always.' },
];

const DEFAULT_ROUTES = [
  { id: 'rt1', from: 'prs', to: 'vyr', type: 'quantum',  color: '#00e5ff', opacity: 0.4,  label: 'Void Thread',    hz: 432 },
  { id: 'rt2', from: 'prs', to: 'sel', type: 'arcane',   color: '#c084fc', opacity: 0.35, label: 'Mythos Weave',   hz: 741 },
  { id: 'rt3', from: 'prs', to: 'aur', type: 'forge',    color: '#fbbf24', opacity: 0.45, label: 'Gold Conduit',   hz: 528 },
  { id: 'rt4', from: 'prs', to: 'bri', type: 'bio',      color: '#39ff14', opacity: 0.3,  label: 'Brine Link',     hz: 396 },
  { id: 'rt5', from: 'prs', to: 'nox', type: 'quantum',  color: '#00e5ff', opacity: 0.25, label: 'Nox Descent',    hz: 285 },
  { id: 'rt6', from: 'aur', to: 'emb', type: 'plasma',   color: '#f97316', opacity: 0.3,  label: 'Ember Reach',    hz: 639 },
  { id: 'rt7', from: 'vyr', to: 'syl', type: 'bio',      color: '#39ff14', opacity: 0.2,  label: 'Sylvan Drift',   hz: 432 },
  { id: 'rt8', from: 'sel', to: 'bri', type: 'arcane',   color: '#c084fc', opacity: 0.2,  label: 'Crystal Loom',   hz: 396 },
];

const DEFAULT_SURVEY = [
  { id: 'sp1', x: 30, y: 18, color: '#00e5ff', label: 'QSV·001', note: 'Quantum field density: high' },
  { id: 'sp2', x: 55, y: 70, color: '#39ff14', label: 'BSV·014', note: 'Bio bloom detected, cycle 3' },
  { id: 'sp3', x: 78, y: 22, color: '#c084fc', label: 'MSV·007', note: 'Narrative thread convergence' },
  { id: 'sp4', x: 88, y: 72, color: '#f97316', label: 'ESV·003', note: 'Thermal probe data nominal' },
  { id: 'sp5', x: 8,  y: 45, color: '#fbbf24', label: 'GSV·011', note: 'Gold-frequency accumulation' },
];

// ── Type → color/glow mapping ──────────────────────────────────────
const TYPE_DEFS = {
  'void-zone':   { color: '#00e5ff', glow: 'rgba(0,229,255,0.5)',    label: 'Void Zone',    stratum: 1 },
  'arcane-node': { color: '#c084fc', glow: 'rgba(192,132,252,0.5)',  label: 'Arcane Node',  stratum: 7 },
  'bio-zone':    { color: '#39ff14', glow: 'rgba(57,255,20,0.5)',    label: 'Bio Zone',     stratum: 2 },
  'forge-node':  { color: '#fbbf24', glow: 'rgba(251,191,36,0.5)',   label: 'Forge Node',   stratum: 5 },
  'plasma-zone': { color: '#f97316', glow: 'rgba(249,115,22,0.5)',   label: 'Plasma Zone',  stratum: 4 },
  'prime-node':  { color: '#fbbf24', glow: 'rgba(251,191,36,0.8)',   label: 'Prime Node',   stratum: 0 },
};

const ROUTE_TYPE_DEFS = {
  'quantum': { color: '#00e5ff', label: 'Quantum Thread' },
  'arcane':  { color: '#c084fc', label: 'Arcane Weave'   },
  'bio':     { color: '#39ff14', label: 'Bio Conduit'    },
  'forge':   { color: '#fbbf24', label: 'Forge Conduit'  },
  'plasma':  { color: '#f97316', label: 'Plasma Arc'     },
};

// ── Load/save from localStorage ────────────────────────────────────
function loadWorld() {
  try {
    const saved = localStorage.getItem('atlas-world');
    if (saved) return JSON.parse(saved);
  } catch {}
  return { regions: DEFAULT_REGIONS, routes: DEFAULT_ROUTES, survey: DEFAULT_SURVEY };
}

function saveWorld(world) {
  try { localStorage.setItem('atlas-world', JSON.stringify(world)); } catch {}
}

// ── ID generator ────────────────────────────────────────────────────
function genId(prefix) {
  return `${prefix}_${Math.random().toString(36).slice(2, 7)}`;
}

// Export
Object.assign(window, { DEFAULT_REGIONS, DEFAULT_ROUTES, DEFAULT_SURVEY, TYPE_DEFS, ROUTE_TYPE_DEFS, loadWorld, saveWorld, genId });
// Backward-compat alias
window.REGIONS = DEFAULT_REGIONS;
