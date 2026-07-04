// myth-controls — ATOM panel control definitions.
//
// A ControlDef is an optional field on SubModuleSpec. When present it
// describes how to build a physical UI widget (fader, knob, XY pad, …)
// on the Instrument Vault panel for that ATOM.
//
// ControlDef is pure data — no renderer dep here.
// The math for taper curves lives in the Instrument Vault (Fyrox layer).

use serde::{Deserialize, Serialize};

/// The widget type to render on the instrument panel for this ATOM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlType {
    Fader,
    Knob,
    Button,
    Toggle,
    XYPad,
    StepSequencer,
    Meter,
    Display,
}

/// The response curve mapping a normalised (0–1) position to a parameter value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaperCurve {
    Linear,
    Logarithmic,
    Exponential,
    SCurve,
}

/// Instructions for building and initialising an ATOM's panel widget.
///
/// Stored as `Option<ControlDef>` on `SubModuleSpec`; `None` means the
/// ATOM has no interactive control (it is a passive processor or data bus).
///
/// `unit_label` and `llm_prompt` are `&'static str` so the 256-entry
/// atlas spec table compiles without any heap allocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlDef {
    pub control_type: ControlType,
    pub range_min: f32,
    pub range_max: f32,
    pub default_value: f32,
    pub taper: TaperCurve,
    /// Short unit string shown beside the value readout (e.g. "Hz", "dB", "%").
    pub unit_label: Option<&'static str>,
    /// Optional LLM generation prompt for procedural widget skin / behaviour.
    pub llm_prompt: Option<&'static str>,
}

impl ControlDef {
    /// Convenience constructor for a simple linear knob in [0, 1].
    pub fn knob(min: f32, max: f32, default: f32) -> Self {
        Self {
            control_type: ControlType::Knob,
            range_min: min,
            range_max: max,
            default_value: default,
            taper: TaperCurve::Linear,
            unit_label: None,
            llm_prompt: None,
        }
    }

    /// Convenience constructor for a vertical fader.
    pub fn fader(min: f32, max: f32, default: f32) -> Self {
        Self {
            control_type: ControlType::Fader,
            range_min: min,
            range_max: max,
            default_value: default,
            taper: TaperCurve::Linear,
            unit_label: None,
            llm_prompt: None,
        }
    }

    pub fn with_unit(mut self, label: &'static str) -> Self {
        self.unit_label = Some(label);
        self
    }

    pub fn with_taper(mut self, taper: TaperCurve) -> Self {
        self.taper = taper;
        self
    }

    pub fn with_prompt(mut self, prompt: &'static str) -> Self {
        self.llm_prompt = Some(prompt);
        self
    }
}
