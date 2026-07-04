/* ═══════════════════════════════════════════════════════════════════
   THE AXIOM THEATER · WorldEngine
   A layer compositor for a procedural world. Each LAYER renders to its
   own offscreen canvas, then is composited onto the main canvas with a
   real blend mode + opacity + illumination — exactly like After Effects.

   ATOMS (pure math, port-friendly to Rust/wgpu):
     · value-noise fBm        → Genesis terrain
     · boids / flocking       → Vorthex swarm
     · L-system turtle        → Emergence flora
     · curl-ish flow field    → Aether
   CAPSULES are the small state bags each ATOM passes around (sims map).
═══════════════════════════════════════════════════════════════════ */
(function () {
  'use strict';

  // ── ATOM: hash value-noise ─────────────────────────────────────────
  function hash(x, y) {
    let n = (x | 0) * 374761393 + (y | 0) * 668265263;
    n = (n ^ (n >> 13)) * 1274126177;
    return ((n ^ (n >> 16)) >>> 0) / 4294967295;
  }
  function vnoise(x, y) {
    const xi = Math.floor(x), yi = Math.floor(y);
    const xf = x - xi, yf = y - yi;
    const u = xf * xf * (3 - 2 * xf), v = yf * yf * (3 - 2 * yf);
    const tl = hash(xi, yi), tr = hash(xi + 1, yi);
    const bl = hash(xi, yi + 1), br = hash(xi + 1, yi + 1);
    return tl * (1 - u) * (1 - v) + tr * u * (1 - v) + bl * (1 - u) * v + br * u * v;
  }
  function fbm(x, y, oct) {
    let a = 0, amp = 0.5, f = 1, norm = 0;
    for (let i = 0; i < (oct || 5); i++) { a += amp * vnoise(x * f, y * f); norm += amp; f *= 2; amp *= 0.5; }
    return a / norm;
  }
  const clamp = (v, a, b) => v < a ? a : v > b ? b : v;
  const lerp = (a, b, t) => a + (b - a) * t;
  const smooth = (e0, e1, x) => { const t = clamp((x - e0) / (e1 - e0), 0, 1); return t * t * (3 - 2 * t); };

  const BLEND = {
    normal: 'source-over', add: 'lighter', dodge: 'lighter',
    screen: 'screen', multiply: 'multiply', overlay: 'overlay', darken: 'darken',
  };

  // ─────────────────────────────────────────────────────────────────
  //  GENESIS · Atlas — top-down planet that forms across aeons
  // ─────────────────────────────────────────────────────────────────
  function genesisInit(w, h) {
    const HW = 220, HH = 220;
    const height = new Float32Array(HW * HH);
    for (let y = 0; y < HH; y++) for (let x = 0; x < HW; x++) {
      // wrap horizontally so the planet can spin seamlessly
      const ang = (x / HW) * Math.PI * 2;
      const nx = Math.cos(ang) * 1.6 + 2, nz = Math.sin(ang) * 1.6 + 2;
      const ny = (y / HH) * 3.2;
      height[y * HW + x] = fbm(nx * 1.7 + ny * 0.0, ny * 1.7 + nz * 0.0 + nz, 6);
    }
    const buf = document.createElement('canvas'); buf.width = HW; buf.height = HH;
    const bctx = buf.getContext('2d');
    const img = bctx.createImageData(HW, HH);
    return {
      HW, HH, height, buf, bctx, img,
      creatures: [], volcano: [], ships: [], debris: [], rings: [],
      hotspots: [{ a: 0.7, r: 0.55 }, { a: 2.3, r: 0.7 }, { a: 4.6, r: 0.4 }],
      eruptBoost: 0, spin: 0,
    };
  }

  function genesisRender(L, ctx, w, h, time, dt, color) {
    const s = L.sim;
    const P = L.params || {};
    const pLand = P.land != null ? P.land : 0.5;
    const pHeat = P.heat != null ? P.heat : 0.4;
    const pLife = P.life != null ? P.life : 0.6;
    const cx = w / 2, cy = h / 2;
    const R = Math.min(w, h) * 0.40;
    s.spin += dt * 0.04;

    // land grows with time; molten early, frozen-cap late
    const landFrac = smooth(0.12, 0.62, time);
    const seaLevel = clamp(lerp(0.92, 0.40, landFrac) - (pLand - 0.5) * 0.28, 0.28, 0.95);
    const molten = 1 - smooth(0.05, 0.20, time);
    const iceFrac = smooth(0.78, 1.0, time);
    const flick = 0.85 + 0.15 * Math.sin(time * 600 + s.spin * 40);

    // recolor the heightmap into the offscreen buffer
    const { HW, HH, height, img } = s;
    const d = img.data;
    const spinOff = (s.spin * 18) % HW;
    for (let y = 0; y < HH; y++) {
      const vy = (y / (HH - 1)) * 2 - 1;        // -1..1 latitude
      const ice = iceFrac * smooth(0.62, 0.92, Math.abs(vy));
      for (let x = 0; x < HW; x++) {
        let sx = x + spinOff; if (sx >= HW) sx -= HW;
        const hgt = height[y * HW + ((sx | 0))];
        const i = (y * HW + x) * 4;
        let r, g, b;
        if (molten > 0.01 && hgt > seaLevel - 0.05 * (1 - molten)) {
          // lava skin while molten
          const t = clamp((hgt - 0.3) * 1.6, 0, 1);
          r = 255 * flick; g = (60 + 150 * t) * flick; b = 20 * t * flick;
          const mm = molten;
          r = lerp(landColor(hgt, seaLevel, ice)[0], r, mm);
          g = lerp(landColor(hgt, seaLevel, ice)[1], g, mm);
          b = lerp(landColor(hgt, seaLevel, ice)[2], b, mm);
        } else if (hgt < seaLevel) {
          const dep = (seaLevel - hgt) / seaLevel;     // 0 shallow .. 1 deep
          r = lerp(34, 6, dep); g = lerp(110, 26, dep); b = lerp(150, 60, dep);
        } else {
          const c = landColor(hgt, seaLevel, ice); r = c[0]; g = c[1]; b = c[2];
        }
        d[i] = r; d[i + 1] = g; d[i + 2] = b; d[i + 3] = 255;
      }
    }
    s.bctx.putImageData(img, 0, 0);

    // draw planet disk
    ctx.save();
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, Math.PI * 2); ctx.clip();
    ctx.imageSmoothingEnabled = true;
    ctx.drawImage(s.buf, cx - R, cy - R, R * 2, R * 2);

    // atmosphere terminator (day/night shading)
    const term = ctx.createLinearGradient(cx - R, 0, cx + R, 0);
    term.addColorStop(0, 'rgba(0,0,0,0.55)');
    term.addColorStop(0.45, 'rgba(0,0,0,0)');
    term.addColorStop(1, 'rgba(0,0,0,0.35)');
    ctx.fillStyle = term; ctx.fillRect(cx - R, cy - R, R * 2, R * 2);

    // sheen
    const sheen = ctx.createRadialGradient(cx - R * 0.35, cy - R * 0.4, R * 0.1, cx, cy, R);
    sheen.addColorStop(0, 'rgba(255,255,255,0.18)');
    sheen.addColorStop(0.4, 'rgba(255,255,255,0.02)');
    sheen.addColorStop(1, 'rgba(0,0,0,0.25)');
    ctx.fillStyle = sheen; ctx.fillRect(cx - R, cy - R, R * 2, R * 2);
    ctx.restore();

    // atmosphere rim glow
    ctx.save();
    ctx.globalCompositeOperation = 'lighter';
    const rim = ctx.createRadialGradient(cx, cy, R * 0.86, cx, cy, R * 1.18);
    rim.addColorStop(0, 'rgba(80,170,255,0)');
    rim.addColorStop(0.7, 'rgba(90,180,255,' + (0.10 + landFrac * 0.18) + ')');
    rim.addColorStop(1, 'rgba(90,180,255,0)');
    ctx.fillStyle = rim; ctx.beginPath(); ctx.arc(cx, cy, R * 1.18, 0, Math.PI * 2); ctx.fill();
    ctx.restore();

    // ── volcanoes (particle fountains) ───────────────────────────────
    const baseEr = clamp(0.55 - time * 0.6, 0, 0.55) + pHeat * 0.45 + s.eruptBoost;
    s.eruptBoost = Math.max(0, s.eruptBoost - dt * 0.9);
    if (baseEr > 0.02) {
      for (const hs of s.hotspots) {
        const px = cx + Math.cos(hs.a + s.spin) * R * hs.r;
        const py = cy + Math.sin(hs.a + s.spin * 0.6) * R * hs.r * 0.7;
        if (Math.random() < baseEr) {
          for (let k = 0; k < 2 + (baseEr * 4 | 0); k++) {
            s.volcano.push({ x: px, y: py, vx: (Math.random() - 0.5) * 60, vy: -40 - Math.random() * 80 * (0.5 + baseEr), life: 1, sz: 1 + Math.random() * 2.4 });
          }
        }
      }
    }
    ctx.save(); ctx.globalCompositeOperation = 'lighter';
    for (let i = s.volcano.length - 1; i >= 0; i--) {
      const p = s.volcano[i]; p.vy += 90 * dt; p.x += p.vx * dt; p.y += p.vy * dt; p.life -= dt * 1.1;
      if (p.life <= 0) { s.volcano.splice(i, 1); continue; }
      const a = p.life;
      ctx.fillStyle = 'rgba(255,' + (90 + 120 * a | 0) + ',30,' + (a * 0.8) + ')';
      ctx.beginPath(); ctx.arc(p.x, p.y, p.sz * (0.6 + a), 0, Math.PI * 2); ctx.fill();
    }
    ctx.restore();

    // ── creatures (life era) — wandering ATOMs ───────────────────────
    const lifeAmt = smooth(0.46, 0.6, time) * (1 - smooth(0.92, 1, time));
    const want = (lifeAmt * 60 * (0.35 + pLife * 1.1)) | 0;
    while (s.creatures.length < want) {
      const a = Math.random() * Math.PI * 2, rr = Math.sqrt(Math.random()) * R * 0.82;
      s.creatures.push({ x: cx + Math.cos(a) * rr, y: cy + Math.sin(a) * rr * 0.92, a: Math.random() * 6.28, sp: 6 + Math.random() * 8, hue: Math.random() < 0.5 ? '#7be36f' : '#d4f062' });
    }
    while (s.creatures.length > want) s.creatures.pop();
    ctx.save(); ctx.globalCompositeOperation = 'lighter';
    for (const c of s.creatures) {
      c.a += (Math.random() - 0.5) * 0.4;
      c.x += Math.cos(c.a) * c.sp * dt; c.y += Math.sin(c.a) * c.sp * dt;
      const dx = c.x - cx, dy = (c.y - cy) / 0.92, dr = Math.hypot(dx, dy);
      if (dr > R * 0.85) { c.a += Math.PI; c.x = cx + dx / dr * R * 0.8; c.y = cy + (dy / dr * R * 0.8) * 0.92; }
      ctx.fillStyle = c.hue; ctx.globalAlpha = 0.9 * lifeAmt;
      ctx.beginPath(); ctx.arc(c.x, c.y, 1.5, 0, Math.PI * 2); ctx.fill();
    }
    ctx.globalAlpha = 1; ctx.restore();

    // ── ships (sampler-spawned alien crash) ──────────────────────────
    ctx.save(); ctx.globalCompositeOperation = 'lighter';
    for (let i = s.ships.length - 1; i >= 0; i--) {
      const sh = s.ships[i]; sh.t += dt;
      const p = clamp(sh.t / sh.dur, 0, 1);
      const x = lerp(sh.x0, sh.x1, p), y = lerp(sh.y0, sh.y1, p);
      // trail
      ctx.strokeStyle = 'rgba(180,255,220,0.5)'; ctx.lineWidth = 2;
      ctx.beginPath(); ctx.moveTo(lerp(sh.x0, sh.x1, Math.max(0, p - 0.08)), lerp(sh.y0, sh.y1, Math.max(0, p - 0.08))); ctx.lineTo(x, y); ctx.stroke();
      ctx.fillStyle = '#d8ffe8'; ctx.beginPath(); ctx.arc(x, y, 3.2, 0, Math.PI * 2); ctx.fill();
      if (p >= 1 && !sh.boom) {
        sh.boom = true; s.rings.push({ x: sh.x1, y: sh.y1, r: 2, life: 1 });
        for (let k = 0; k < 40; k++) { const a = Math.random() * 6.28, v = 30 + Math.random() * 140; s.debris.push({ x: sh.x1, y: sh.y1, vx: Math.cos(a) * v, vy: Math.sin(a) * v, life: 1, sz: 1 + Math.random() * 2 }); }
      }
      if (sh.boom) s.ships.splice(i, 1);
    }
    for (let i = s.rings.length - 1; i >= 0; i--) {
      const rg = s.rings[i]; rg.r += 220 * dt; rg.life -= dt * 1.2;
      if (rg.life <= 0) { s.rings.splice(i, 1); continue; }
      ctx.strokeStyle = 'rgba(160,255,210,' + (rg.life * 0.7) + ')'; ctx.lineWidth = 2;
      ctx.beginPath(); ctx.arc(rg.x, rg.y, rg.r, 0, Math.PI * 2); ctx.stroke();
    }
    for (let i = s.debris.length - 1; i >= 0; i--) {
      const p = s.debris[i]; p.vy += 60 * dt; p.x += p.vx * dt; p.y += p.vy * dt; p.life -= dt * 0.9;
      if (p.life <= 0) { s.debris.splice(i, 1); continue; }
      ctx.fillStyle = 'rgba(200,255,160,' + p.life + ')';
      ctx.beginPath(); ctx.arc(p.x, p.y, p.sz, 0, Math.PI * 2); ctx.fill();
    }
    ctx.restore();
  }
  function landColor(h, sea, ice) {
    const e = clamp((h - sea) / (1 - sea), 0, 1); // 0 coast .. 1 peak
    let r, g, b;
    if (e < 0.12) { r = 196; g = 178; b = 120; }            // beach
    else if (e < 0.5) { const t = (e - 0.12) / 0.38; r = lerp(86, 60, t); g = lerp(140, 96, t); b = lerp(58, 44, t); } // green→forest
    else { const t = (e - 0.5) / 0.5; r = lerp(96, 150, t); g = lerp(82, 150, t); b = lerp(60, 150, t); } // rock→snow
    if (ice > 0.01) { r = lerp(r, 235, ice); g = lerp(g, 245, ice); b = lerp(b, 255, ice); }
    return [r, g, b];
  }
  function genesisEvent(L, name) {
    const s = L.sim;
    if (name === 'ERUPT' || name === 'QUAKE') s.eruptBoost = Math.min(1.4, s.eruptBoost + 0.9);
    if (name === 'SHIP') {
      const w = L._w, h = L._h, cx = w / 2, cy = h / 2, R = Math.min(w, h) * 0.4;
      const a = Math.PI * 0.25 + Math.random() * 0.4;
      s.ships.push({ x0: -40, y0: cy - R * 0.9, x1: cx + (Math.random() - 0.5) * R, y1: cy + R * 0.2, t: 0, dur: 1.1, boom: false });
    }
  }

  // ─────────────────────────────────────────────────────────────────
  //  VORTHEX · boids swarm forming tactical patterns
  // ─────────────────────────────────────────────────────────────────
  function vorthexInit(w, h) {
    const N = 130, agents = [];
    for (let i = 0; i < N; i++) agents.push({ x: Math.random() * w, y: Math.random() * h, vx: (Math.random() - 0.5) * 40, vy: (Math.random() - 0.5) * 40 });
    return { agents, formation: null, formT: 0, params: { sep: 0.5, ali: 0.5, coh: 0.5 } };
  }
  function vorthexRender(L, ctx, w, h, time, dt, color) {
    const s = L.sim, A = s.agents;
    const P = L.params || s.params;
    const sepW = 1.6 * (0.4 + P.sep), aliW = 0.9 * (0.3 + P.ali), cohW = 0.7 * (0.3 + P.coh);
    const per = 46, maxSpd = 120;
    s.formT = Math.max(0, s.formT - dt * 0.35);
    for (let i = 0; i < A.length; i++) {
      const a = A[i];
      let sx = 0, sy = 0, ax = 0, ay = 0, cxx = 0, cyy = 0, n = 0;
      for (let j = 0; j < A.length; j++) {
        if (i === j) continue; const b = A[j];
        const dx = a.x - b.x, dy = a.y - b.y; const d2 = dx * dx + dy * dy;
        if (d2 < per * per && d2 > 0.0001) {
          const d = Math.sqrt(d2); sx += dx / d; sy += dy / d;
          ax += b.vx; ay += b.vy; cxx += b.x; cyy += b.y; n++;
        }
      }
      if (n > 0) {
        a.vx += sx * sepW; a.vy += sy * sepW;
        a.vx += (ax / n - a.vx) * 0.02 * aliW * 10 * dt * 6;
        a.vy += (ay / n - a.vy) * 0.02 * aliW * 10 * dt * 6;
        a.vx += ((cxx / n) - a.x) * cohW * 0.02; a.vy += ((cyy / n) - a.y) * cohW * 0.02;
      }
      // formation pull
      if (s.formation) {
        const tgt = s.formation[i % s.formation.length];
        a.vx += (tgt.x - a.x) * (0.8 + s.formT) * 0.06;
        a.vy += (tgt.y - a.y) * (0.8 + s.formT) * 0.06;
      }
      // gentle center gravity + wander
      a.vx += (w / 2 - a.x) * 0.0008; a.vy += (h / 2 - a.y) * 0.0008;
      a.vx += (Math.random() - 0.5) * 4; a.vy += (Math.random() - 0.5) * 4;
      const sp = Math.hypot(a.vx, a.vy); if (sp > maxSpd) { a.vx = a.vx / sp * maxSpd; a.vy = a.vy / sp * maxSpd; }
      a.x += a.vx * dt; a.y += a.vy * dt;
      if (a.x < 0) a.x += w; if (a.x > w) a.x -= w; if (a.y < 0) a.y += h; if (a.y > h) a.y -= h;
    }
    // hive web (links between near agents)
    ctx.save();
    ctx.globalCompositeOperation = 'lighter';
    ctx.lineWidth = 0.6; ctx.strokeStyle = 'rgba(0,200,255,0.13)';
    for (let i = 0; i < A.length; i += 1) {
      const a = A[i];
      for (let j = i + 1; j < A.length; j++) {
        const b = A[j]; const dx = a.x - b.x, dy = a.y - b.y; const d2 = dx * dx + dy * dy;
        if (d2 < 52 * 52) { ctx.globalAlpha = (1 - d2 / (52 * 52)) * 0.5; ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke(); }
      }
    }
    ctx.globalAlpha = 1;
    // chevrons
    for (const a of A) {
      const ang = Math.atan2(a.vy, a.vx);
      ctx.save(); ctx.translate(a.x, a.y); ctx.rotate(ang);
      ctx.fillStyle = color || '#ff4dbb';
      ctx.beginPath(); ctx.moveTo(6, 0); ctx.lineTo(-4, 3); ctx.lineTo(-2, 0); ctx.lineTo(-4, -3); ctx.closePath(); ctx.fill();
      ctx.restore();
    }
    ctx.restore();
  }
  function vorthexEvent(L, name) {
    const s = L.sim, w = L._w, h = L._h, cx = w / 2, cy = h / 2;
    const N = s.agents.length; const pts = [];
    const make = (fn) => { for (let i = 0; i < N; i++) pts.push(fn(i, N)); s.formation = pts; s.formT = 1; };
    if (name === 'SWARM') {                 // ring
      const R = Math.min(w, h) * 0.34; make((i, n) => ({ x: cx + Math.cos(i / n * 6.283) * R, y: cy + Math.sin(i / n * 6.283) * R }));
    } else if (name === 'FORMATION') {       // wedge / arrow
      make((i, n) => { const row = i % 13, col = (i / 13) | 0; return { x: cx - 120 + col * 26 + row * 0, y: cy + (row - 6) * 16 + col * 0 }; });
    } else if (name === 'GRID') {
      make((i, n) => { const c = 13; return { x: cx - 150 + (i % c) * 25, y: cy - 80 + ((i / c) | 0) * 25 }; });
    } else if (name === 'SCATTER') { s.formation = null; }
  }

  // ─────────────────────────────────────────────────────────────────
  //  EMERGENCE FLORA · L-system turtle, animated growth (loops)
  // ─────────────────────────────────────────────────────────────────
  function floraInit(w, h) { return { segs: null, grown: 0, age: 0, angle: 25, depth: 4, seed: 1, blossoms: [] }; }
  function floraBuild(s, w, h) {
    // axiom F, rule F -> FF+[+F-F-F]-[-F+F+F]
    let str = 'F';
    for (let i = 0; i < s.depth; i++) {
      let out = '';
      for (const ch of str) out += ch === 'F' ? 'FF+[+F-F-F]-[-F+F+F]' : ch;
      str = out;
      if (str.length > 14000) break;
    }
    const segs = []; const tips = [];
    const stack = []; let x = w / 2, y = h * 0.98, ang = -Math.PI / 2;
    const step = Math.min(w, h) / Math.pow(2.0, s.depth) * 2.2;
    const da = s.angle * Math.PI / 180;
    let depth = 0;
    for (const ch of str) {
      if (ch === 'F') { const nx = x + Math.cos(ang) * step, ny = y + Math.sin(ang) * step; segs.push({ x1: x, y1: y, x2: nx, y2: ny, d: depth }); x = nx; y = ny; }
      else if (ch === '+') ang += da;
      else if (ch === '-') ang -= da;
      else if (ch === '[') { stack.push({ x, y, ang, depth }); depth++; }
      else if (ch === ']') { const st = stack.pop(); tips.push({ x, y }); x = st.x; y = st.y; ang = st.ang; depth = st.depth; }
    }
    s.segs = segs; s.tips = tips; s.total = segs.length;
  }
  function floraRender(L, ctx, w, h, time, dt, color) {
    const s = L.sim;
    const P = L.params || {};
    const ang = 18 + (P.mid != null ? P.mid : 0.4) * 24;
    const depth = 3 + Math.round((P.low != null ? P.low : 0.5) * 2);
    if (!s.segs || s._ang !== ang || s._depth !== depth) { s.angle = ang; s.depth = depth; floraBuild(s, w, h); s._ang = ang; s._depth = depth; s.grown = 0; }
    const growthRate = 0.18 + (P.hi != null ? P.hi : 0.5) * 0.5;
    s.grown += dt * growthRate; if (s.grown > 1.25) s.grown = 0; // loop
    s.age += dt;
    const show = clamp(s.grown, 0, 1) * s.total;
    ctx.save();
    ctx.lineCap = 'round';
    for (let i = 0; i < s.segs.length && i < show; i++) {
      const sg = s.segs[i];
      const lifeIn = clamp(show - i, 0, 1);
      const dd = sg.d;
      const wdt = Math.max(0.5, 4 - dd * 0.8);
      // trunk brown → branch bio-green → tip glow
      const t = clamp(dd / 5, 0, 1);
      const r = lerp(120, 70, t), g = lerp(80, 220, t), b = lerp(40, 90, t);
      ctx.strokeStyle = 'rgba(' + (r | 0) + ',' + (g | 0) + ',' + (b | 0) + ',' + (0.5 + 0.5 * lifeIn) + ')';
      ctx.lineWidth = wdt;
      ctx.beginPath(); ctx.moveTo(sg.x1, sg.y1); ctx.lineTo(sg.x2, sg.y2); ctx.stroke();
    }
    // blossoms at tips, pulse
    ctx.globalCompositeOperation = 'lighter';
    const tipsShow = clamp((s.grown - 0.7) / 0.3, 0, 1);
    if (s.tips) for (let i = 0; i < s.tips.length; i++) {
      const tp = s.tips[i];
      const pulse = 0.6 + 0.4 * Math.sin(s.age * 3 + i);
      const rad = 2.2 * tipsShow * pulse;
      if (rad <= 0.1) continue;
      ctx.fillStyle = 'rgba(251,191,36,' + (0.7 * tipsShow) + ')';
      ctx.beginPath(); ctx.arc(tp.x, tp.y, rad, 0, Math.PI * 2); ctx.fill();
    }
    ctx.restore();
  }
  function floraEvent(L, name) { if (name === 'BLOOM') { L.sim.grown = 0; } }

  // ─────────────────────────────────────────────────────────────────
  //  AETHER · flow-field particles (background wash)
  // ─────────────────────────────────────────────────────────────────
  function aetherInit(w, h) {
    const N = 360, p = [];
    for (let i = 0; i < N; i++) p.push({ x: Math.random() * w, y: Math.random() * h, px: 0, py: 0 });
    return { p, t: 0 };
  }
  function aetherRender(L, ctx, w, h, time, dt, color) {
    const s = L.sim; s.t += dt;
    const P = L.params || {};
    const scl = 0.004 + (P.mid != null ? P.mid : 0.4) * 0.004;
    const spd = 26 + (P.hi != null ? P.hi : 0.5) * 60;
    ctx.save();
    ctx.globalCompositeOperation = 'lighter';
    ctx.lineWidth = 1;
    for (const a of s.p) {
      const ang = fbm(a.x * scl, a.y * scl + s.t * 0.15, 3) * Math.PI * 4;
      a.px = a.x; a.py = a.y;
      a.x += Math.cos(ang) * spd * dt; a.y += Math.sin(ang) * spd * dt;
      if (a.x < 0 || a.x > w || a.y < 0 || a.y > h) { a.x = Math.random() * w; a.y = Math.random() * h; a.px = a.x; a.py = a.y; continue; }
      const la = 0.18 + (P.low != null ? P.low : 0.5) * 0.42;
      ctx.strokeStyle = color ? hexA(color, la) : 'rgba(0,200,255,' + la + ')';
      ctx.beginPath(); ctx.moveTo(a.px, a.py); ctx.lineTo(a.x, a.y); ctx.stroke();
    }
    ctx.restore();
  }
  function hexA(hex, a) {
    const m = hex.replace('#', ''); const n = parseInt(m.length === 3 ? m.split('').map(c => c + c).join('') : m, 16);
    return 'rgba(' + ((n >> 16) & 255) + ',' + ((n >> 8) & 255) + ',' + (n & 255) + ',' + a + ')';
  }

  const KINDS = {
    genesis: { init: genesisInit, render: genesisRender, event: genesisEvent },
    vorthex: { init: vorthexInit, render: vorthexRender, event: vorthexEvent },
    flora: { init: floraInit, render: floraRender, event: floraEvent },
    aether: { init: aetherInit, render: aetherRender, event: function () {} },
  };

  // ─────────────────────────────────────────────────────────────────
  //  ENGINE
  // ─────────────────────────────────────────────────────────────────
  class WorldEngine {
    constructor(canvas) {
      this.canvas = canvas; this.ctx = canvas.getContext('2d');
      this.w = canvas.width; this.h = canvas.height;
      this.layers = [];           // [{id,kind,visible,opacity,blend,illum,solo,color,params}]
      this._L = {};               // per-id live layer state {sim, params, color, _w,_h}
      this.time = 0.32; this.playing = false; this.rate = 0.045; this.loop = [0, 1];
      this.shake = 0; this.flash = 0;
      this.onTime = null;
      this._raf = null; this._last = 0;
      this._frame = this._frame.bind(this);
    }
    resize(w, h) {
      this.w = this.canvas.width = w; this.h = this.canvas.height = h;
      for (const id in this._L) { const L = this._L[id]; L._w = w; L._h = h; const k = KINDS[L.kind]; if (k) L.sim = k.init(w, h); }
    }
    setLayers(layers) {
      this.layers = layers;
      for (const ly of layers) {
        let L = this._L[ly.id];
        if (!L || L.kind !== ly.kind) {
          const k = KINDS[ly.kind] || KINDS.aether;
          const cnv = document.createElement('canvas'); cnv.width = this.w; cnv.height = this.h;
          L = this._L[ly.id] = { kind: ly.kind, sim: k.init(this.w, this.h), canvas: cnv, ctx: cnv.getContext('2d'), _w: this.w, _h: this.h };
        }
        L.params = ly.params || {}; L.color = ly.color;
      }
    }
    setTime(t) { this.time = clamp(t, 0, 1); if (this.onTime) this.onTime(this.time); }
    scrub(d) { this.setTime(this.time + d); }
    setPlaying(p) { this.playing = p; }
    setLoop(a, b) { this.loop = [a, b]; }
    fireEvent(name, payload) {
      // global FX
      if (name === 'QUAKE') this.shake = 1;
      if (name === 'FLASH') this.flash = 1;
      if (name === 'FREEZE') this.playing = false;
      for (const ly of this.layers) {
        const L = this._L[ly.id]; if (!L) continue; L.params = ly.params || {}; L.color = ly.color;
        const k = KINDS[ly.kind]; if (k && k.event) k.event(L, name);
      }
    }
    start() { if (this._raf) return; this._last = performance.now(); this._raf = requestAnimationFrame(this._frame); }
    stop() { if (this._raf) cancelAnimationFrame(this._raf); this._raf = null; }
    _frame(now) {
      let dt = (now - this._last) / 1000; this._last = now; if (dt > 0.05) dt = 0.05;
      // transport advances aeon-time, looping
      if (this.playing) {
        let nt = this.time + this.rate * dt * 4;
        const [a, b] = this.loop;
        if (nt >= b) nt = a + (nt - b);
        if (nt < a) nt = b - (a - nt);
        this.setTime(clamp(nt, 0, 1));
      }
      this._render(dt, now / 1000);
      this._raf = requestAnimationFrame(this._frame);
    }
    _render(dt, now) {
      const ctx = this.ctx, w = this.w, h = this.h;
      this.shake = Math.max(0, this.shake - dt * 3);
      this.flash = Math.max(0, this.flash - dt * 2.2);
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.clearRect(0, 0, w, h);
      ctx.fillStyle = '#04060b'; ctx.fillRect(0, 0, w, h);
      const sk = this.shake * 8;
      if (sk > 0.1) ctx.setTransform(1, 0, 0, 1, (Math.random() - 0.5) * sk, (Math.random() - 0.5) * sk);

      const anySolo = this.layers.some(l => l.solo);
      for (const ly of this.layers) {
        if (!ly.visible) continue; if (anySolo && !ly.solo) continue;
        const L = this._L[ly.id]; if (!L) continue;
        L.params = ly.params || {}; L.color = ly.color; L._w = w; L._h = h;
        const lc = L.ctx; lc.setTransform(1, 0, 0, 1, 0, 0); lc.clearRect(0, 0, w, h);
        const k = KINDS[ly.kind]; if (k) k.render(L, lc, w, h, this.time, dt, ly.color);
        ctx.globalAlpha = clamp(ly.opacity * (ly.illum != null ? ly.illum : 1), 0, 1);
        ctx.globalCompositeOperation = BLEND[ly.blend] || 'source-over';
        ctx.drawImage(L.canvas, 0, 0);
      }
      ctx.globalAlpha = 1; ctx.globalCompositeOperation = 'source-over';
      if (this.flash > 0.01) { ctx.fillStyle = 'rgba(255,255,255,' + (this.flash * 0.7) + ')'; ctx.fillRect(0, 0, w, h); }
      ctx.setTransform(1, 0, 0, 1, 0, 0);
    }
  }

  window.WorldEngine = WorldEngine;
  window.WORLD_KINDS = Object.keys(KINDS);
})();
