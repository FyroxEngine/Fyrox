/* =====================================================
   CONTROLS — knobs, sliders, jacks, switches, pads, displays
   All components also exported to window for cross-script use.
   ===================================================== */

const { useState, useRef, useEffect, useCallback, useMemo } = React;

/* ---------- helpers ---------- */
const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));
const lerp  = (a, b, t) => a + (b - a) * t;
const fmt   = (v, d = 2) => v.toFixed(d);

/* turn a 0..1 value into an angle around a knob */
function valToAngle(v, sweep = 270) {
  const a = -sweep / 2 + v * sweep;  // -135..+135
  return a;
}

/* hook: drag-to-set value (vertical drag) */
function useDragValue(initial, { min = 0, max = 1, sensitivity = 200 } = {}) {
  const [value, setValue] = useState(initial);
  const startRef = useRef({ y: 0, v: 0 });

  const onPointerDown = useCallback((e) => {
    e.preventDefault();
    e.currentTarget.setPointerCapture?.(e.pointerId);
    startRef.current = { y: e.clientY, v: value };
    const move = (ev) => {
      const dy = startRef.current.y - ev.clientY;
      const range = max - min;
      const next = clamp(startRef.current.v + (dy / sensitivity) * range, min, max);
      setValue(next);
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }, [value, min, max, sensitivity]);

  return [value, setValue, onPointerDown];
}

/* =====================================================
   KNOB — primary radial control
   variants: 'classic' | 'arc' | 'dotted' | 'forge' | 'ringed' | 'pip'
   ===================================================== */
function Knob({
  value: controlled,
  defaultValue = 0.5,
  onChange,
  min = 0, max = 1,
  variant = "classic",
  size = 64,
  channel = "cool",       // glow color key
  label, unit, decimals = 2,
  ticks = 0,              // tick count (0 = none)
  bipolar = false,
  sweep = 270,
  showValue = true,
}) {
  const isControlled = controlled !== undefined;
  const [internal, setInternal, onPointerDown] = useDragValue(defaultValue, { min, max });
  const value = isControlled ? controlled : internal;
  const setValue = (v) => { isControlled ? onChange?.(v) : setInternal(v); onChange?.(v); };

  // override drag to call onChange
  const handlePointerDown = useCallback((e) => {
    e.preventDefault();
    e.currentTarget.setPointerCapture?.(e.pointerId);
    const startY = e.clientY;
    const startV = value;
    const move = (ev) => {
      const dy = startY - ev.clientY;
      const next = clamp(startV + (dy / 200) * (max - min), min, max);
      setValue(next);
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }, [value, min, max]);

  const norm = (value - min) / (max - min);
  const angle = valToAngle(norm, sweep);
  const glow = `var(--glow-${channel})`;
  const accent = `var(--signal-${channel})`;

  const tickEls = useMemo(() => {
    if (!ticks) return null;
    return Array.from({ length: ticks }).map((_, i) => {
      const t = i / (ticks - 1);
      const a = -sweep/2 + t * sweep;
      const active = bipolar
        ? (t >= 0.5 ? norm >= t : norm <= t)
        : norm >= t - 0.5/ticks;
      return (
        <div key={i} className="kb-tick" style={{
          transform: `rotate(${a}deg) translateY(${-(size/2) + 2}px)`,
          background: active ? `var(--signal-${channel})` : "rgba(255,255,255,0.08)",
          boxShadow: active ? `0 0 6px rgba(${glow.replace('var(--glow-','').replace(')','')}, 0.8)` : "none",
        }}/>
      );
    });
  }, [ticks, sweep, norm, bipolar, channel, size]);

  // arc indicator (svg)
  const arcRadius = size/2 - 4;
  const arcCirc = 2 * Math.PI * arcRadius;
  const arcLen = (sweep / 360) * arcCirc;
  const arcOffset = bipolar
    ? arcLen * Math.abs(0.5 - norm)
    : arcLen * (1 - norm);
  const arcStart = bipolar ? arcLen * 0.5 : 0;

  return (
    <div className="kb-wrap" style={{ width: size + 12 }}>
      <div
        className={`kb kb-${variant}`}
        style={{ width: size, height: size, "--kb-accent": accent, "--kb-glow": glow }}
        onPointerDown={handlePointerDown}
      >
        {/* ARC variant — neon ring outside */}
        {(variant === "arc" || variant === "ringed" || variant === "forge") && (
          <svg className="kb-arc" viewBox={`0 0 ${size} ${size}`} width={size} height={size}>
            <circle
              cx={size/2} cy={size/2} r={arcRadius}
              fill="none"
              stroke="rgba(255,255,255,0.05)"
              strokeWidth={variant === "forge" ? 4 : 2.5}
            />
            <circle
              cx={size/2} cy={size/2} r={arcRadius}
              fill="none"
              stroke={accent}
              strokeWidth={variant === "forge" ? 4 : 2.5}
              strokeLinecap="round"
              strokeDasharray={`${arcLen - arcOffset} ${arcCirc}`}
              strokeDashoffset={-arcStart}
              transform={`rotate(${-90 - sweep/2 + (bipolar ? sweep/2 : 0)} ${size/2} ${size/2})`}
              style={{ filter: `drop-shadow(0 0 4px ${accent})` }}
            />
          </svg>
        )}

        {/* DOTTED variant — dotted outer ring */}
        {variant === "dotted" && (
          <div className="kb-dotted-ring" />
        )}

        {/* tick marks */}
        {ticks > 0 && <div className="kb-ticks">{tickEls}</div>}

        {/* the knob cap itself */}
        <div className="kb-cap" style={{ transform: `rotate(${angle}deg)` }}>
          {variant === "pip" ? (
            <div className="kb-pip" />
          ) : (
            <div className="kb-indicator" />
          )}
          {variant === "forge" && <div className="kb-knurl-ring" />}
        </div>

        {/* center dot for some variants */}
        {(variant === "ringed") && <div className="kb-center-dot" />}
      </div>

      {label && <div className="kb-label engrave">{label}</div>}
      {showValue && (
        <div className="kb-value readout">
          {fmt(bipolar ? (value - (min+max)/2) * 2 / (max-min) : value, decimals)}{unit}
        </div>
      )}
    </div>
  );
}

/* =====================================================
   FADER / SLIDER — vertical or horizontal travel
   ===================================================== */
function Fader({
  value: controlled, defaultValue = 0.5, onChange,
  min = 0, max = 1, height = 120, width = 28,
  channel = "cool", label, orientation = "vertical",
  showScale = true,
}) {
  const [internal, setInternal] = useState(defaultValue);
  const value = controlled !== undefined ? controlled : internal;
  const setValue = (v) => { setInternal(v); onChange?.(v); };
  const trackRef = useRef(null);
  const isVert = orientation === "vertical";

  const handlePointerDown = useCallback((e) => {
    e.preventDefault();
    e.currentTarget.setPointerCapture?.(e.pointerId);
    const update = (ev) => {
      const r = trackRef.current.getBoundingClientRect();
      const t = isVert
        ? 1 - clamp((ev.clientY - r.top) / r.height, 0, 1)
        : clamp((ev.clientX - r.left) / r.width, 0, 1);
      setValue(min + t * (max - min));
    };
    update(e);
    const move = (ev) => update(ev);
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }, [min, max, isVert]);

  const norm = (value - min) / (max - min);
  const accent = `var(--signal-${channel})`;

  return (
    <div className={`fader fader-${orientation}`} style={{
      width: isVert ? width + 28 : height + 28,
      "--fader-accent": accent,
    }}>
      {label && <div className="fader-label engrave">{label}</div>}
      <div className="fader-body" style={{
        height: isVert ? height : width,
        width:  isVert ? width  : height,
      }}>
        {showScale && (
          <div className="fader-scale">
            {Array.from({ length: 11 }).map((_, i) => (
              <div key={i} className={`fader-tick ${i % 5 === 0 ? "fader-tick-major" : ""}`} />
            ))}
          </div>
        )}
        <div
          ref={trackRef}
          className="fader-track"
          onPointerDown={handlePointerDown}
        >
          <div className="fader-fill" style={{
            [isVert ? "height" : "width"]: `${norm * 100}%`,
          }}/>
          <div className="fader-cap" style={{
            [isVert ? "bottom" : "left"]: `calc(${norm * 100}% - 8px)`,
          }}>
            <div className="fader-cap-line" />
          </div>
        </div>
      </div>
    </div>
  );
}

/* =====================================================
   JACK — patch point. Click to "patch", glows when active.
   ===================================================== */
function Jack({ label, channel = "cool", active = false, kind = "out", onClick }) {
  const accent = `var(--signal-${channel})`;
  return (
    <div className="jack-wrap">
      <div
        className={`jack ${active ? "jack-active" : ""}`}
        style={{ "--jack-accent": accent }}
        onClick={onClick}
        data-kind={kind}
      >
        <div className="jack-rim" />
        <div className="jack-hole" />
        {active && <div className="jack-glow" />}
      </div>
      {label && <div className="jack-label engrave">{label}</div>}
    </div>
  );
}

/* =====================================================
   SWITCH — toggle, latch, or 3-position
   ===================================================== */
function Switch({ value, onChange, positions = 2, labels, channel = "cool" }) {
  const [internal, setInternal] = useState(0);
  const v = value !== undefined ? value : internal;
  const set = (n) => { setInternal(n); onChange?.(n); };
  return (
    <div className={`sw sw-${positions}`} onClick={() => set((v + 1) % positions)}
         style={{ "--sw-accent": `var(--signal-${channel})` }}>
      <div className="sw-track">
        {Array.from({ length: positions }).map((_, i) => (
          <div key={i} className={`sw-pos ${v === i ? "active" : ""}`} />
        ))}
      </div>
      <div className="sw-thumb" style={{ transform: `translateY(${(v / (positions - 1)) * 100}%)` }} />
      {labels && <div className="sw-labels">
        {labels.map((l,i) => <span key={i} className={v === i ? "active":""}>{l}</span>)}
      </div>}
    </div>
  );
}

/* =====================================================
   BUTTON / TRIGGER PAD
   ===================================================== */
function Pad({ label, channel = "cool", lit = false, onClick, size = 36 }) {
  return (
    <button
      className={`pad ${lit ? "pad-lit" : ""}`}
      style={{ width: size, height: size, "--pad-accent": `var(--signal-${channel})` }}
      onClick={onClick}
    >
      {label && <span className="pad-label">{label}</span>}
    </button>
  );
}

/* trigger gate — square with red/green LED */
function GateBtn({ label, lit = false, channel = "life", onClick }) {
  return (
    <button className={`gatebtn ${lit ? "lit" : ""}`} onClick={onClick}
            style={{ "--gate-accent": `var(--signal-${channel})` }}>
      <div className="gatebtn-led" />
      {label && <span className="engrave">{label}</span>}
    </button>
  );
}

/* =====================================================
   DIGITAL READOUT — segmented display
   ===================================================== */
function Readout({ value, label, channel = "cool", width = 80, align = "right" }) {
  return (
    <div className="ro-wrap" style={{ width, "--ro-accent": `var(--signal-${channel})` }}>
      {label && <div className="ro-label engrave">{label}</div>}
      <div className="ro-screen">
        <div className="ro-bg-segments">88:88</div>
        <div className="ro-value" style={{ textAlign: align }}>{value}</div>
      </div>
    </div>
  );
}

/* =====================================================
   LED — tiny status indicator
   ===================================================== */
function LED({ on = false, channel = "life", size = 6 }) {
  return (
    <span className={`led ${on ? "on" : ""}`}
      style={{
        width: size, height: size,
        "--led-accent": `var(--signal-${channel})`,
        "--led-glow": `var(--glow-${channel})`,
      }}/>
  );
}

/* =====================================================
   STEP SEQUENCER ROW — 16 steps
   ===================================================== */
function StepRow({ steps = 16, pattern, onToggle, current = -1, channel = "cool" }) {
  const [internal] = useState(() => pattern || Array.from({ length: steps }, () => Math.random() > 0.65));
  const data = pattern || internal;
  return (
    <div className="step-row" style={{ "--step-accent": `var(--signal-${channel})` }}>
      {data.map((on, i) => (
        <button key={i}
          className={`step ${on ? "on" : ""} ${current === i ? "current":""}`}
          onClick={() => onToggle?.(i)}
        >
          <span className="step-led" />
        </button>
      ))}
    </div>
  );
}

/* =====================================================
   XY PAD — joystick-like 2D control
   ===================================================== */
function XYPad({ size = 120, channel = "myth", x: xC, y: yC, onChange, label }) {
  const [pos, setPos] = useState({ x: xC ?? 0.5, y: yC ?? 0.5 });
  const ref = useRef(null);

  const handle = (e) => {
    if (!ref.current) return;
    const r = ref.current.getBoundingClientRect();
    const x = clamp((e.clientX - r.left) / r.width, 0, 1);
    const y = clamp((e.clientY - r.top) / r.height, 0, 1);
    setPos({ x, y });
    onChange?.({ x, y });
  };

  const onDown = (e) => {
    e.currentTarget.setPointerCapture?.(e.pointerId);
    handle(e);
    const move = (ev) => handle(ev);
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  return (
    <div className="xy-wrap">
      {label && <div className="engrave xy-label">{label}</div>}
      <div ref={ref} className="xy-pad mat-screen"
           style={{ width: size, height: size,
                    "--xy-accent": `var(--signal-${channel})` }}
           onPointerDown={onDown}>
        <div className="xy-grid" />
        <div className="xy-crosshair" style={{ left: `${pos.x*100}%`, top: `${pos.y*100}%` }}>
          <div className="xy-dot" />
        </div>
        <div className="xy-trace" style={{ left: `${pos.x*100}%`}} />
        <div className="xy-trace xy-trace-h" style={{ top: `${pos.y*100}%`}} />
      </div>
    </div>
  );
}

/* expose to other scripts */
Object.assign(window, {
  Knob, Fader, Jack, Switch, Pad, GateBtn,
  Readout, LED, StepRow, XYPad,
  clamp, lerp, fmt,
});
