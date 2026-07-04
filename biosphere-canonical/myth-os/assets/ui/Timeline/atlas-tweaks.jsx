/* Quantum Atlas — Tweaks
   Embedded in the main app root via <AtlasTweaks/>.
   TweaksPanel handles the host protocol internally.
*/

const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "glowIntensity": 1.0,
  "animSpeed": 1.0,
  "showCompass": true,
  "panelOpacity": 0.92,
  "routeOpacity": 0.4,
  "accentColor": "quantum"
}/*EDITMODE-END*/;

function AtlasTweaks() {
  const [tweaks, setTweak] = useTweaks(TWEAK_DEFAULTS);

  // Broadcast tweaks globally so map/panels can read them
  React.useEffect(() => {
    window.__atlasTweaks = tweaks;
  }, [tweaks]);

  return (
    <TweaksPanel title="Atlas Tweaks">
      <TweakSection label="Map Style" />
      <TweakRadio
        label="Accent"
        value={tweaks.accentColor}
        options={[
          { value: 'quantum', label: 'Quantum' },
          { value: 'mythos',  label: 'Mythos'  },
          { value: 'gold',    label: 'Gold'    },
        ]}
        onChange={v => setTweak('accentColor', v)}
      />
      <TweakSection label="Rendering" />
      <TweakSlider
        label="Glow Intensity"
        value={tweaks.glowIntensity}
        min={0} max={2} step={0.1}
        onChange={v => setTweak('glowIntensity', v)}
      />
      <TweakSlider
        label="Anim Speed"
        value={tweaks.animSpeed}
        min={0} max={3} step={0.1}
        onChange={v => setTweak('animSpeed', v)}
      />
      <TweakSlider
        label="Route Opacity"
        value={tweaks.routeOpacity}
        min={0} max={1} step={0.05}
        onChange={v => setTweak('routeOpacity', v)}
      />
      <TweakSlider
        label="Panel Opacity"
        value={tweaks.panelOpacity}
        min={0.5} max={1} step={0.01}
        onChange={v => setTweak('panelOpacity', v)}
      />
      <TweakSection label="Overlays" />
      <TweakToggle
        label="Show Compass"
        value={tweaks.showCompass}
        onChange={v => setTweak('showCompass', v)}
      />
    </TweaksPanel>
  );
}

Object.assign(window, { AtlasTweaks, TWEAK_DEFAULTS });
