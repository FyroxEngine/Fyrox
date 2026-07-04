/* global React */
const { useState: useStateV, useRef: useRefV, useEffect: useEffectV, useMemo: useMemoV } = React;

/* ═══════════════════════════════════════════════════════════════════
   AETHYR VISUALIZERS — animated SVG-based scopes & meters.
   All accept `channel` to color, and most are self-animating.
═══════════════════════════════════════════════════════════════════ */

function useAnim(callback, fps = 60) {
  useEffectV(() => {
    let raf, last = 0;
    const tick = (t) => {
      if (t - last >= 1000/fps) { callback(t); last = t; }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);
}

/* ─── Oscilloscope — sliding waveform ─────────────────────────── */
function Scope({ width = 280, height = 100, channel = 'audio', source = 'sine' }) {
  const [phase, setPhase] = useStateV(0);
  const ch = channelVars(channel);
  useAnim(() => setPhase(p => (p + 0.015) % (Math.PI * 4)));
  const points = [];
  const N = 100;
  for (let i = 0; i < N; i++) {
    const x = (i / (N-1)) * width;
    const t = (i / N) * Math.PI * 4 + phase;
    let y = 0;
    if (source === 'sine')      y = Math.sin(t);
    else if (source === 'fm')   y = Math.sin(t + Math.sin(t*2)*0.5);
    else if (source === 'noise')y = Math.sin(t)*0.6 + (Math.random()-0.5)*0.5;
    else                         y = Math.sin(t)*0.7 + Math.sin(t*3)*0.2;
    points.push(`${x},${height/2 - y * (height*0.35)}`);
  }
  return (
    <div style={{
      width, height, background: 'var(--glass-screen)',
      borderRadius: 4, border: '1px solid rgba(0,0,0,0.6)',
      boxShadow: 'inset 0 2px 6px rgba(0,0,0,0.7), 0 1px 0 rgba(255,255,255,0.04)',
      position: 'relative', overflow: 'hidden',
    }}>
      <div style={{ position: 'absolute', inset: 0, background: 'var(--glass-grid)', opacity: 0.6 }} />
      <svg width={width} height={height} style={{ position: 'absolute', inset: 0 }}>
        <line x1={0} y1={height/2} x2={width} y2={height/2} stroke="rgba(120,180,255,0.12)" strokeDasharray="2 4"/>
        <polyline points={points.join(' ')} fill="none" stroke={ch.c} strokeWidth="1.5"
          style={{ filter: `drop-shadow(0 0 3px ${ch.g})` }} />
      </svg>
      <div style={{ position: 'absolute', inset: 0, background: 'var(--glass-scanlines)', opacity: 0.5, pointerEvents: 'none' }} />
    </div>
  );
}

/* ─── Spectrum — animated bars ───────────────────────────────── */
function Spectrum({ width = 280, height = 100, bars = 32, channel = 'poly' }) {
  const [vals, setVals] = useStateV(() => Array.from({length: bars}, (_, i) => 0.4 + Math.random() * 0.5));
  const ch = channelVars(channel);
  useAnim(() => {
    setVals(prev => prev.map((v, i) => {
      const target = (Math.sin(Date.now()/300 + i*0.4) * 0.3 + 0.5) * (1 - i/bars * 0.6) + Math.random()*0.15;
      return v + (target - v) * 0.18;
    }));
  });
  const bw = (width - 6) / bars - 1;
  return (
    <div style={{
      width, height, background: 'var(--glass-screen)',
      borderRadius: 4, border: '1px solid rgba(0,0,0,0.6)',
      boxShadow: 'inset 0 2px 6px rgba(0,0,0,0.7)',
      padding: 3, display: 'flex', alignItems: 'flex-end', gap: 1,
      overflow: 'hidden', position: 'relative',
    }}>
      <div style={{ position: 'absolute', inset: 0, background: 'var(--glass-scanlines)', opacity: 0.5 }} />
      {vals.map((v, i) => (
        <div key={i} style={{
          width: bw, height: `${Math.max(2, v * 100)}%`,
          background: `linear-gradient(180deg,
            ${ch.c} 0%, ${ch.c}cc 60%, ${ch.c}66 100%)`,
          boxShadow: `0 0 4px ${ch.g}`,
          borderRadius: '1px 1px 0 0',
          opacity: 0.85,
        }} />
      ))}
    </div>
  );
}

/* ─── XY Pad — phase scope ───────────────────────────────────── */
function XYPad({ size = 140, channel = 'mythos', live = true }) {
  const [trail, setTrail] = useStateV([]);
  const ch = channelVars(channel);
  useAnim((t) => {
    if (!live) return;
    const x = size/2 + Math.cos(t/420) * size * 0.32 * (1 + Math.sin(t/1300) * 0.3);
    const y = size/2 + Math.sin(t/380 + Math.sin(t/700)*1.2) * size * 0.32;
    setTrail(prev => [...prev.slice(-40), {x, y}]);
  });
  return (
    <div style={{
      width: size, height: size,
      background: 'var(--glass-screen)',
      borderRadius: 4, border: '1px solid rgba(0,0,0,0.6)',
      boxShadow: 'inset 0 2px 6px rgba(0,0,0,0.7)',
      position: 'relative', overflow: 'hidden',
    }}>
      <div style={{ position: 'absolute', inset: 0, background: 'var(--glass-grid)', opacity: 0.5 }} />
      <div style={{ position: 'absolute', left: '50%', top: 0, bottom: 0, width: 1, background: 'rgba(120,180,255,0.15)' }} />
      <div style={{ position: 'absolute', top: '50%', left: 0, right: 0, height: 1, background: 'rgba(120,180,255,0.15)' }} />
      <svg width={size} height={size} style={{ position: 'absolute', inset: 0 }}>
        {trail.length > 1 && (
          <polyline points={trail.map(p => `${p.x},${p.y}`).join(' ')}
            fill="none" stroke={ch.c} strokeWidth="1.2" strokeLinecap="round"
            style={{ filter: `drop-shadow(0 0 3px ${ch.g})`, opacity: 0.7 }} />
        )}
        {trail.length > 0 && (
          <circle cx={trail[trail.length-1].x} cy={trail[trail.length-1].y} r={3}
            fill={ch.c} style={{ filter: `drop-shadow(0 0 6px ${ch.g})` }} />
        )}
      </svg>
    </div>
  );
}

/* ─── VU Meter — horizontal bar with peak hold ──────────────── */
function VUMeter({ width = 200, channel = 'audio', label, source = 'pulse' }) {
  const [level, setLevel] = useStateV(0.4);
  const [peak, setPeak] = useStateV(0.4);
  const ch = channelVars(channel);
  useAnim((t) => {
    const target = source === 'pulse'
      ? 0.5 + Math.sin(t/280) * 0.3 + (Math.random()-0.5)*0.1
      : 0.3 + Math.random() * 0.6;
    setLevel(prev => prev + (Math.max(0, Math.min(1, target)) - prev) * 0.25);
    setPeak(prev => Math.max(prev * 0.99, level));
  });
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, width }}>
      {label && <div className="cap-label" style={{ width: 28 }}>{label}</div>}
      <div style={{
        flex: 1, height: 14,
        background: 'var(--panel-inset)',
        borderRadius: 2,
        border: '1px solid rgba(0,0,0,0.6)',
        boxShadow: 'inset 0 1px 3px rgba(0,0,0,0.7)',
        position: 'relative', overflow: 'hidden',
      }}>
        <div style={{
          position: 'absolute', left: 0, top: 0, bottom: 0,
          width: `${level * 100}%`,
          background: `linear-gradient(90deg,
            var(--bio) 0%, var(--bio) 60%,
            var(--gold) 75%, var(--ember) 92%, var(--rose) 100%)`,
          boxShadow: `0 0 6px ${ch.g}`,
          transition: 'width 50ms linear',
        }} />
        <div style={{
          position: 'absolute', top: 0, bottom: 0,
          left: `calc(${peak * 100}% - 2px)`,
          width: 2, background: '#fff',
          boxShadow: '0 0 4px rgba(255,255,255,0.6)',
          opacity: 0.7,
        }} />
        {/* tick marks */}
        {[0.25,0.5,0.75].map(p => (
          <div key={p} style={{
            position: 'absolute', top: 0, bottom: 0,
            left: `${p*100}%`, width: 1,
            background: 'rgba(0,0,0,0.4)',
          }}/>
        ))}
      </div>
    </div>
  );
}

/* ─── Polar / Radar plot ─────────────────────────────────────── */
function Polar({ size = 140, channel = 'gold', petals = 7 }) {
  const [phase, setPhase] = useStateV(0);
  const ch = channelVars(channel);
  useAnim(() => setPhase(p => p + 0.005));
  const cx = size/2, cy = size/2, R = size/2 - 8;
  const N = 90;
  const points = [];
  for (let i = 0; i <= N; i++) {
    const a = (i/N) * Math.PI * 2;
    const r = R * (0.45 + 0.35 * Math.abs(Math.sin(a*petals/2 + phase)) + 0.08*Math.sin(a*petals*2 + phase*3));
    points.push(`${cx + r*Math.cos(a)},${cy + r*Math.sin(a)}`);
  }
  return (
    <div style={{
      width: size, height: size,
      background: 'var(--glass-screen)',
      borderRadius: '50%',
      border: '1px solid rgba(0,0,0,0.6)',
      boxShadow: 'inset 0 2px 8px rgba(0,0,0,0.7), 0 0 0 1px rgba(120,180,255,0.1)',
      position: 'relative', overflow: 'hidden',
    }}>
      <svg width={size} height={size}>
        {[0.33, 0.66, 1].map((s,i) => (
          <circle key={i} cx={cx} cy={cy} r={R*s} fill="none" stroke="rgba(120,180,255,0.1)" strokeDasharray="2 3"/>
        ))}
        {[0,45,90,135].map(a => (
          <line key={a} x1={cx} y1={cy}
            x2={cx + R*Math.cos(a*Math.PI/180)} y2={cy + R*Math.sin(a*Math.PI/180)}
            stroke="rgba(120,180,255,0.08)" />
        ))}
        <polygon points={points.join(' ')} fill={`${ch.c}22`} stroke={ch.c} strokeWidth="1.2"
          style={{ filter: `drop-shadow(0 0 4px ${ch.g})` }} />
        <circle cx={cx} cy={cy} r={2} fill={ch.c} style={{ filter: `drop-shadow(0 0 4px ${ch.g})` }} />
      </svg>
    </div>
  );
}

/* ─── Curve Editor — envelope/LFO with handles ───────────────── */
function CurveEditor({ width = 280, height = 100, channel = 'cv', preset = 'adsr' }) {
  const ch = channelVars(channel);
  const path = useMemoV(() => {
    const p = [];
    if (preset === 'adsr') {
      p.push([0, height-4]);
      p.push([width*0.15, 8]);
      p.push([width*0.32, height*0.35]);
      p.push([width*0.7, height*0.4]);
      p.push([width-4, height-4]);
    } else if (preset === 'lfo') {
      for (let i=0;i<=40;i++){
        const x = i/40 * width;
        const y = height/2 - Math.sin(i/40 * Math.PI*2.5) * (height*0.35);
        p.push([x,y]);
      }
    } else {
      p.push([0, height-4],[width*0.4, height*0.2],[width*0.6, height*0.7],[width-4,8]);
    }
    return p;
  }, [preset, width, height]);
  const handles = preset === 'adsr' ? path : [path[0], path[Math.floor(path.length/2)], path[path.length-1]];
  const d = preset === 'lfo'
    ? `M ${path.map(p => p.join(',')).join(' L ')}`
    : `M ${path[0][0]} ${path[0][1]} ` + path.slice(1).map(p => `L ${p[0]} ${p[1]}`).join(' ');
  return (
    <div style={{
      width, height,
      background: 'var(--glass-screen)',
      borderRadius: 4, border: '1px solid rgba(0,0,0,0.6)',
      boxShadow: 'inset 0 2px 6px rgba(0,0,0,0.7)',
      position: 'relative', overflow: 'hidden',
    }}>
      <div style={{ position: 'absolute', inset: 0, background: 'var(--glass-grid)', opacity: 0.5 }} />
      <svg width={width} height={height}>
        <path d={`${d} L ${width-4} ${height-4} L 0 ${height-4} Z`} fill={`${ch.c}11`} />
        <path d={d} fill="none" stroke={ch.c} strokeWidth="1.6"
          style={{ filter: `drop-shadow(0 0 3px ${ch.g})` }} />
        {handles.map((h,i) => (
          <g key={i}>
            <circle cx={h[0]} cy={h[1]} r={5} fill="#0a0c12" stroke={ch.c} strokeWidth="1.5"
              style={{ filter: `drop-shadow(0 0 4px ${ch.g})` }} />
            <circle cx={h[0]} cy={h[1]} r={1.5} fill={ch.c} />
          </g>
        ))}
      </svg>
    </div>
  );
}

/* ─── Spectrogram — time/frequency heatmap ──────────────────── */
function Spectrogram({ width = 280, height = 100 }) {
  const ref = useRefV(null);
  useEffectV(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    canvas.width = width; canvas.height = height;
    const cols = 80;
    const rows = 24;
    let t = 0;
    let raf;
    const draw = () => {
      // shift left
      const img = ctx.getImageData(width/cols, 0, width - width/cols, height);
      ctx.putImageData(img, 0, 0);
      // new col
      const x = width - width/cols;
      for (let y = 0; y < rows; y++) {
        const v = Math.abs(Math.sin(y*0.4 + t*0.05) * Math.sin(t*0.02 + y*0.1) + Math.random()*0.3) * (1 - y/rows*0.7);
        const hue = 200 + (1-y/rows) * 80;
        const lit = Math.max(0, Math.min(1, v));
        ctx.fillStyle = `hsla(${hue}, 80%, ${20 + lit*40}%, ${lit})`;
        ctx.fillRect(x, height - (y+1) * height/rows, width/cols + 1, height/rows + 1);
      }
      t++;
      raf = requestAnimationFrame(draw);
    };
    draw();
    return () => cancelAnimationFrame(raf);
  }, []);
  return (
    <div style={{
      width, height,
      background: 'var(--glass-screen)',
      borderRadius: 4, border: '1px solid rgba(0,0,0,0.6)',
      boxShadow: 'inset 0 2px 6px rgba(0,0,0,0.7)',
      overflow: 'hidden', position: 'relative',
    }}>
      <canvas ref={ref} style={{ display: 'block', width, height }} />
      <div style={{ position: 'absolute', inset: 0, background: 'var(--glass-scanlines)', opacity: 0.4, pointerEvents: 'none' }} />
    </div>
  );
}

/* ─── ParticleField ────────────────────────────────────────── */
function ParticleField({ size = 180, channel = 'mythos', count = 60 }) {
  const ref = useRefV(null);
  const ch = channelVars(channel);
  useEffectV(() => {
    const canvas = ref.current;
    const ctx = canvas.getContext('2d');
    canvas.width = size; canvas.height = size;
    const particles = Array.from({length: count}, () => ({
      x: Math.random() * size, y: Math.random() * size,
      vx: 0, vy: 0, life: Math.random(),
    }));
    let raf;
    const draw = (t) => {
      ctx.fillStyle = 'rgba(8, 10, 18, 0.12)';
      ctx.fillRect(0,0,size,size);
      const cx = size/2 + Math.cos(t*0.001)*10;
      const cy = size/2 + Math.sin(t*0.0013)*10;
      particles.forEach(p => {
        const dx = cx - p.x, dy = cy - p.y;
        const d = Math.sqrt(dx*dx+dy*dy)+1;
        // swirl + attract
        const swirl = 0.5;
        p.vx += (-dy/d - dx/d*0.05) * swirl;
        p.vy += (dx/d - dy/d*0.05) * swirl;
        p.vx *= 0.93; p.vy *= 0.93;
        p.x += p.vx; p.y += p.vy;
        p.life -= 0.005;
        if (p.life < 0 || p.x<0||p.x>size||p.y<0||p.y>size) {
          p.x = cx + (Math.random()-0.5)*40;
          p.y = cy + (Math.random()-0.5)*40;
          p.vx = (Math.random()-0.5)*2; p.vy = (Math.random()-0.5)*2;
          p.life = 0.7+Math.random()*0.3;
        }
        const cssMatch = ch.c.match(/var\((--[^)]+)\)/);
        const color = ch.c.startsWith('#') ? ch.c : '#c084fc';
        ctx.fillStyle = `rgba(192, 132, 252, ${p.life * 0.7})`;
        ctx.fillRect(p.x, p.y, 1.5, 1.5);
      });
      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(raf);
  }, []);
  return (
    <div style={{
      width: size, height: size,
      background: 'radial-gradient(circle at 50% 50%, #0a0820 0%, #03050a 100%)',
      borderRadius: 4, border: '1px solid rgba(0,0,0,0.6)',
      boxShadow: `inset 0 2px 8px rgba(0,0,0,0.7), inset 0 0 32px ${ch.g}`,
      overflow: 'hidden',
    }}>
      <canvas ref={ref} style={{ display: 'block' }} />
    </div>
  );
}

/* ─── Timeline strip — playhead + keyframes ──────────────── */
function Timeline({ width = 480, channel = 'gold', frames = 96, playing = true }) {
  const [t, setT] = useStateV(0);
  const ch = channelVars(channel);
  useAnim(() => { if (playing) setT(prev => (prev + 0.4) % frames); });
  const keyframes = [12, 24, 38, 56, 70, 84];
  return (
    <div style={{
      width, height: 36,
      background: 'var(--panel-inset)',
      border: '1px solid rgba(0,0,0,0.6)',
      borderRadius: 3,
      boxShadow: 'inset 0 1px 3px rgba(0,0,0,0.7)',
      position: 'relative',
      display: 'flex', alignItems: 'center',
    }}>
      {/* tick marks */}
      <div style={{ position:'absolute', inset: 0, display:'flex', alignItems:'flex-end' }}>
        {Array.from({length: frames+1}).map((_,i) => (
          <div key={i} style={{
            width: width/frames, height: i%8===0 ? 10 : i%4===0 ? 6 : 3,
            borderLeft: '1px solid rgba(120,180,255,0.12)',
          }}/>
        ))}
      </div>
      {/* keyframes */}
      {keyframes.map((kf,i) => (
        <div key={i} style={{
          position: 'absolute', left: `${(kf/frames)*100}%`,
          top: '50%', transform: 'translate(-50%,-50%) rotate(45deg)',
          width: 8, height: 8, background: 'var(--gold)',
          boxShadow: '0 0 6px var(--gold-glow)',
        }}/>
      ))}
      {/* playhead */}
      <div style={{
        position:'absolute', left: `${(t/frames)*100}%`,
        top: -2, bottom: -2, width: 2,
        background: ch.c, boxShadow: `0 0 8px ${ch.g}`,
        transform: 'translateX(-50%)',
      }}>
        <div style={{
          position:'absolute', top: -4, left: '50%', transform:'translateX(-50%)',
          width: 10, height: 6, background: ch.c,
          clipPath: 'polygon(50% 100%, 0 0, 100% 0)',
          filter: `drop-shadow(0 0 4px ${ch.g})`,
        }}/>
      </div>
      {/* frame counter */}
      <div style={{
        position:'absolute', right: 6, top: 2,
        fontFamily: 'var(--font-code)', fontSize: 8,
        letterSpacing:'0.15em', color: ch.c, textShadow:`0 0 4px ${ch.g}`,
      }}>F {String(Math.floor(t)).padStart(3,'0')}/{frames}</div>
    </div>
  );
}

/* ─── MIDI Grid — piano roll strip ────────────────────────── */
function MIDIGrid({ width = 280, height = 80, channel = 'midi' }) {
  const ch = channelVars(channel);
  const notes = [
    {p:0,t:0.05,l:0.1},{p:3,t:0.15,l:0.05},{p:5,t:0.22,l:0.08},
    {p:7,t:0.32,l:0.15},{p:5,t:0.5,l:0.05},{p:3,t:0.58,l:0.12},
    {p:8,t:0.74,l:0.06},{p:10,t:0.82,l:0.14},
  ];
  const PITCHES = 12;
  return (
    <div style={{
      width, height,
      background: 'var(--glass-screen)',
      borderRadius: 4, border: '1px solid rgba(0,0,0,0.6)',
      boxShadow: 'inset 0 2px 6px rgba(0,0,0,0.7)',
      position: 'relative', overflow: 'hidden',
    }}>
      {Array.from({length: PITCHES}).map((_,i) => (
        <div key={i} style={{
          position: 'absolute', left: 0, right: 0,
          top: `${(i/PITCHES)*100}%`, height: `${100/PITCHES}%`,
          background: i%2 ? 'rgba(255,255,255,0.015)' : 'transparent',
          borderTop: '1px solid rgba(120,180,255,0.05)',
        }}/>
      ))}
      {notes.map((n,i) => (
        <div key={i} style={{
          position:'absolute',
          left: `${n.t*100}%`, top: `${(1-n.p/PITCHES)*100 - 8}%`,
          width: `${n.l*100}%`, height: `${100/PITCHES - 2}%`,
          background: ch.c, borderRadius: 1,
          boxShadow: `0 0 4px ${ch.g}`, opacity: 0.85,
        }}/>
      ))}
      {/* playhead */}
      <Playhead width={width} channel={channel}/>
    </div>
  );
}
function Playhead({ width, channel }) {
  const ch = channelVars(channel);
  const [t, setT] = useStateV(0);
  useAnim(() => setT(prev => (prev + 0.003) % 1));
  return (
    <div style={{
      position:'absolute', top:0, bottom:0, width: 1.5,
      left: `${t*100}%`, background: ch.c,
      boxShadow: `0 0 6px ${ch.g}`,
    }}/>
  );
}

/* ─── Waveform (static) — file viz ────────────────────────── */
function Waveform({ width = 280, height = 60, channel = 'audio', dense = 80 }) {
  const ch = channelVars(channel);
  const samples = useMemoV(() =>
    Array.from({length: dense}, (_, i) => {
      const env = Math.sin(i / dense * Math.PI);
      return env * (0.6 + 0.4 * Math.sin(i * 0.4)) * (0.7 + Math.random()*0.3);
    }), [dense]);
  return (
    <div style={{
      width, height,
      background: 'var(--glass-screen)',
      borderRadius: 4, border: '1px solid rgba(0,0,0,0.6)',
      boxShadow: 'inset 0 2px 6px rgba(0,0,0,0.7)',
      display: 'flex', alignItems: 'center', gap: 1,
      padding: '0 4px',
      position: 'relative', overflow: 'hidden',
    }}>
      {samples.map((s,i) => (
        <div key={i} style={{
          width: (width-8)/dense - 1,
          height: `${Math.abs(s)*100}%`,
          background: ch.c, opacity: 0.7,
          boxShadow: `0 0 2px ${ch.g}`,
          borderRadius: 0.5,
        }}/>
      ))}
    </div>
  );
}

Object.assign(window, {
  useAnim, Scope, Spectrum, XYPad, VUMeter, Polar, CurveEditor,
  Spectrogram, ParticleField, Timeline, MIDIGrid, Waveform,
});
