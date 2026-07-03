Quantum Genesis — 16 Wire Types
================================

DAT — Data         Universal fallback. Connects to any type.
CTL — Control      Boolean / gate / trigger signals.
AUD — Audio        Waveform and sample streams.
NAR — Narrative    Story / text / lore content.
TMP — Temporal     Time / tick / clock signals.
AGT — Agent        Agent instruction and state packets.
VIS — Visual       Image / render / shader data.
SPA — Spatial      3D / voxel / coordinate data.
BHV — Behavioral   Emotion / drive / decision signals.
SOC — Social       Relationship / faction / reputation.
ENR — Energy       Power and resource flow.
IDN — Identity     B-DNA / lineage / covenant payloads.
EVT — Event        Cosmic bus events.
AST — Asset        File / binary / media references.
MET — Meta         Schema / type / structure definitions.
LGC — Logic        Boolean expression / rule streams.

COMPATIBILITY RULES
  source.wire_type must match target.wire_type, OR
  source.wire_type is DAT (universal fallback).
  No other cross-type connections are permitted.

B-DNA flows through IDN wires as read-only payload.
MIDI/musical data flows through AUD + CTL combinations.
ACTOR identity and covenants flow through IDN wires.
