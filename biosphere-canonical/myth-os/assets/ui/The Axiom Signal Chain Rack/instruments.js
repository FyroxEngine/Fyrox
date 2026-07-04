/* ═══════════════════════════════════════════════════════════════════
   AXIOM · INSTRUMENT REGISTRY
   1 Genesis = 16 Mythos Containers (Core Instruments)
   each Instrument = 16 Containers (EuroSynth modules)
   each Container  = up to 16 Capsule controls (the smallest narrative unit)
   Deterministic generation — stable across reloads, ports cleanly to Rust.
═══════════════════════════════════════════════════════════════════ */
(function () {
  'use strict';

  // wire-type → accent (Genesis signal taxonomy)
  const WIRE = {
    SPA: { c: '#00bfff', n: 'Spatial' }, BHV: { c: '#b06bff', n: 'Behavior' },
    IDN: { c: '#fbbf24', n: 'Identity' }, DAT: { c: '#39ff14', n: 'Data' },
    TMP: { c: '#ff2db5', n: 'Temporal' }, NAR: { c: '#8b5cf6', n: 'Narrative' },
    AUD: { c: '#f97316', n: 'Audio' }, LGC: { c: '#14b8a6', n: 'Logic' },
    ENR: { c: '#fb7185', n: 'Energy' }, SOC: { c: '#6366f1', n: 'Social' },
    EVT: { c: '#ef4444', n: 'Event' }, VIS: { c: '#5DCAA5', n: 'Visual' },
    AST: { c: '#f59e0b', n: 'Asset' }, CTL: { c: '#94a3b8', n: 'Control' },
  };

  // ── 16 Mythos Containers ───────────────────────────────────────────
  // theaterKind links an instrument to a live Theater layer (or null).
  const INSTRUMENTS = [
    { id: 'genesis', name: 'GENESIS · ATLAS', tag: 'ATL', ch: 'gd', theaterKind: 'genesis', motto: 'Worlds from constants.', status: 'live' },
    { id: 'vorthex', name: 'VORTHEX SWARM', tag: 'VTX', ch: 'mg', theaterKind: 'vorthex', motto: 'One mind, many wings.', status: 'live' },
    { id: 'flora', name: 'EMERGENCE FLORA', tag: 'FLR', ch: 'gn', theaterKind: 'flora', motto: 'Rules become gardens.', status: 'live' },
    { id: 'aether', name: 'AETHER FIELD', tag: 'ATH', ch: 'cy', theaterKind: 'aether', motto: 'The medium of all flow.', status: 'live' },
    { id: 'mythos', name: 'MYTHOS WEAVE', tag: 'MYT', ch: 'vi', theaterKind: null, motto: 'Every story, already mapped.', status: 'live' },
    { id: 'agora', name: 'AGORA EXCHANGE', tag: 'AGR', ch: 'gd', theaterKind: null, motto: 'Value finds its level.', status: 'live' },
    { id: 'persona', name: 'PERSONA FORGE', tag: 'PRS', ch: 'mg', theaterKind: null, motto: 'From mark, a mind.', status: 'live' },
    { id: 'chronos', name: 'CHRONOS LOOM', tag: 'CHR', ch: 'cy', theaterKind: null, motto: 'Each tick a fate.', status: 'live' },
    { id: 'nebula', name: 'NEBULA GRID', tag: 'NBL', ch: 'cy', theaterKind: null, motto: 'Stars are servers.', status: 'live' },
    { id: 'thauma', name: 'THAUMA TABLE', tag: 'THM', ch: 'vi', theaterKind: null, motto: 'Cast, and it is seen.', status: 'live' },
    { id: 'lattice', name: 'VOID LATTICE', tag: 'VLT', ch: 'gn', theaterKind: null, motto: 'Spacetime, woven taut.', status: 'beta' },
    { id: 'codex', name: 'CODEX MNEMON', tag: 'CDX', ch: 'gd', theaterKind: null, motto: 'What was, persists.', status: 'beta' },
    { id: 'augur', name: 'AUGUR ORACLE', tag: 'AUG', ch: 'mg', theaterKind: null, motto: 'Many futures, one branch.', status: 'beta' },
    { id: 'hydra', name: 'HYDRA FLOW', tag: 'HYD', ch: 'cy', theaterKind: null, motto: 'Tend the living current.', status: 'beta' },
    { id: 'concord', name: 'CONCORD MASTER', tag: 'CNC', ch: 'gd', theaterKind: null, motto: 'As above, so woven.', status: 'live' },
    { id: 'nexus', name: 'NEXUS ROUTER', tag: 'NXS', ch: 'cy', theaterKind: null, motto: 'Bind, weave, sever.', status: 'live' },
  ];

  // ── MOLECULE archetypes — control layouts for a EuroSynth container ─
  const K = (label, wire) => ({ type: 'knob', label, wire });
  const F = (label, wire) => ({ type: 'fader', label, wire });
  const T = (label, wire) => ({ type: 'toggle', label, wire });
  const J = (label, wire, dir) => ({ type: 'jack', label, wire, dir });
  const S = (label, wire, opts) => ({ type: 'select', label, wire, opts });
  const PADS = (n, wire) => ({ type: 'pads', label: 'MATRIX', wire, n });

  const ARCH = [
    { t: 'FILTER BANK', tag: 'FLT', wire: 'AUD', set: [K('HI', 'SPA'), K('MID', 'LGC'), K('LOW', 'DAT'), K('RES', 'TMP'), F('CUTOFF', 'AUD'), J('IN', 'AUD', 'in'), J('OUT', 'AUD', 'out')] },
    { t: 'PAD MATRIX', tag: 'MTX', wire: 'EVT', set: [PADS(16, 'EVT'), K('VEL', 'CTL'), J('TRIG', 'EVT', 'out')] },
    { t: 'LFO CLUSTER', tag: 'LFO', wire: 'TMP', set: [K('RATE', 'TMP'), K('DEPTH', 'ENR'), K('PHASE', 'TMP'), S('SHAPE', 'TMP', ['SIN', 'TRI', 'SQR', 'RND']), T('SYNC', 'CTL'), J('CV', 'TMP', 'out')] },
    { t: 'EQ STRIP', tag: 'EQ', wire: 'AUD', set: [K('LOW', 'DAT'), K('MID', 'LGC'), K('HIGH', 'SPA'), F('GAIN', 'ENR'), T('MUTE', 'CTL'), T('SOLO', 'IDN')] },
    { t: 'ENVELOPE', tag: 'ENV', wire: 'ENR', set: [K('ATK', 'ENR'), K('DEC', 'ENR'), K('SUS', 'DAT'), K('REL', 'TMP'), J('GATE', 'EVT', 'in'), J('ENV', 'ENR', 'out')] },
    { t: 'ROUTER', tag: 'RTR', wire: 'CTL', set: [J('A', 'SPA', 'in'), J('B', 'BHV', 'in'), J('C', 'DAT', 'in'), J('X', 'AUD', 'out'), J('Y', 'NAR', 'out'), K('XFADE', 'CTL'), S('MODE', 'LGC', ['SUM', 'FLIP', 'SPLIT'])] },
    { t: 'SEQ LANE', tag: 'SEQ', wire: 'TMP', set: [PADS(8, 'TMP'), K('TEMPO', 'TMP'), K('SWING', 'NAR'), J('CLK', 'TMP', 'in')] },
    { t: 'MIX BUS', tag: 'MIX', wire: 'AUD', set: [F('A', 'SPA'), F('B', 'BHV'), K('PAN', 'CTL'), K('SEND', 'VIS'), T('MUTE', 'CTL'), J('SUM', 'AUD', 'out')] },
    { t: 'OSC CORE', tag: 'OSC', wire: 'ENR', set: [K('PITCH', 'TMP'), K('FINE', 'TMP'), K('FOLD', 'ENR'), S('WAVE', 'ENR', ['SAW', 'SIN', 'PUL', 'NSE']), J('FM', 'ENR', 'in'), J('OUT', 'AUD', 'out')] },
    { t: 'SHAPER', tag: 'SHP', wire: 'VIS', set: [K('DRIVE', 'ENR'), K('BIAS', 'DAT'), K('MIX', 'VIS'), F('LEVEL', 'VIS'), J('IN', 'VIS', 'in'), J('OUT', 'VIS', 'out')] },
  ];

  // ── deterministic pseudo-random ─────────────────────────────────────
  function rng(seed) { let s = seed >>> 0; return () => { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967295; }; }
  const hashStr = (str) => { let h = 2166136261; for (let i = 0; i < str.length; i++) { h ^= str.charCodeAt(i); h = Math.imul(h, 16777619); } return h >>> 0; };

  const RANGES = [[0, 10], [0, 100], [-1, 1], [0, 1], [20, 20000], [0, 127], [-12, 12], [1, 16]];
  const UNITS = ['', '%', 'hz', 'db', 'st', 'ms', 'x'];

  // build the 16 containers of an instrument
  function containers(instId) {
    const inst = INSTRUMENTS.find(i => i.id === instId) || INSTRUMENTS[0];
    const out = [];
    for (let ci = 0; ci < 16; ci++) {
      const arch = ARCH[ci % ARCH.length];
      const rnd = rng(hashStr(instId + ':' + ci) || 1);
      const controls = [];
      arch.set.forEach((spec, idx) => {
        if (spec.type === 'pads') {
          for (let p = 0; p < spec.n; p++) controls.push(mkCtl(instId, ci, controls.length, { type: 'pad', label: 'P' + (p + 1), wire: spec.wire }, rnd));
        } else {
          controls.push(mkCtl(instId, ci, controls.length, spec, rnd));
        }
      });
      out.push({
        id: instId + '.c' + ci, n: ci, instId,
        name: arch.tag + '·' + String(ci + 1).padStart(2, '0'),
        archetype: arch.t, wire: arch.wire, color: WIRE[arch.wire].c,
        controls,
      });
    }
    return out;
  }

  function mkCtl(instId, ci, idx, spec, rnd) {
    const r = RANGES[(hashStr(instId + ci + idx) % RANGES.length)];
    const u = UNITS[(hashStr(spec.label + idx) % UNITS.length)];
    return {
      id: instId + '.' + ci + '.' + idx,
      type: spec.type, label: spec.label, wire: spec.wire,
      color: WIRE[spec.wire].c, dir: spec.dir || null, opts: spec.opts || null,
      value: spec.type === 'toggle' ? (rnd() > 0.5) : (spec.type === 'select' ? 0 : rnd()),
      min: r[0], max: r[1], unit: u,
      script: defaultScript(spec, r),
    };
  }
  function defaultScript(spec, r) {
    if (spec.type === 'knob' || spec.type === 'fader') return 'out = lerp(' + r[0] + ', ' + r[1] + ', v)';
    if (spec.type === 'toggle') return 'out = v ? 1 : 0';
    if (spec.type === 'pad') return 'on trigger → emit(EVT)';
    if (spec.type === 'jack') return spec.dir === 'in' ? 'route ← signal' : 'route → signal';
    if (spec.type === 'select') return 'out = opts[v]';
    return 'out = v';
  }

  window.AXIOM_INSTRUMENTS = INSTRUMENTS;
  window.AXIOM_WIRE = WIRE;
  window.axiomContainers = containers;
})();
