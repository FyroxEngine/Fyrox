/// myth-daw session demo
///
/// Boots a small Quantum Quill session, runs the transport for 2 bars,
/// and prints every WirePacket emitted — the same packets the Theater
/// would receive in a real myth-os session.
///
/// Run with:  cargo run --example session_demo -p myth-daw

use myth_daw::{
    arrangement::{Arrangement, AutomationLane},
    clip::Clip,
    mixer::Mixer,
    session::{Scene, Session},
    track::{Track, TrackKind},
    transport::Transport,
    wire::{AutomationValue, ClipEvent, MixerLevel, TransportTick},
};
use uuid::Uuid;

fn main() {
    println!("╔══════════════════════════════════════════════════╗");
    println!("║         MYTH-DAW  —  SESSION DEMO               ║");
    println!("║  Quantum Quill Narrative Workstation             ║");
    println!("╚══════════════════════════════════════════════════╝\n");

    // ── 1. Transport ──────────────────────────────────────────────────────────
    let mut transport = Transport {
        bpm: 120.0,
        ..Default::default()
    };
    transport.play();
    println!("▶  Transport started  —  {:.1} BPM", transport.bpm);

    // ── 2. Session ────────────────────────────────────────────────────────────
    let mut session = Session::new();

    let char_track   = Track::new("Marcus — Character Arc", TrackKind::Character);
    let emotion_track = Track::new("Sorrow Layer",          TrackKind::Emotion);
    let env_track    = Track::new("Café Interior",          TrackKind::Environment);

    let char_id   = char_track.id;
    let emo_id    = emotion_track.id;
    let env_id    = env_track.id;

    session.add_track(char_track);
    session.add_track(emotion_track);
    session.add_track(env_track);
    session.add_scene("Opening — Arrival");
    session.add_scene("Rising Tension");

    // Place clips in the grid
    let capsule_a = Uuid::new_v4();
    let capsule_b = Uuid::new_v4();
    let capsule_c = Uuid::new_v4();

    session.set_clip(0, 0, Clip::new(capsule_a, "Marcus enters café"));
    session.set_clip(0, 1, Clip::new(capsule_b, "Sorrow — low intensity"));
    session.set_clip(0, 2, Clip::new(capsule_c, "Café ambience"));
    session.set_clip(1, 0, Clip::new(capsule_a, "Marcus sees her"));
    session.set_clip(1, 1, Clip::new(capsule_b, "Sorrow — peak"));

    println!("\n📋  Session grid:  {} scenes × {} tracks",
        session.scene_count(), session.track_count());

    // ── 3. Arrangement ────────────────────────────────────────────────────────
    let mut arrangement = Arrangement::new();

    let char_arr   = Track::new("Marcus — Character Arc", TrackKind::Character);
    let char_arr_id = char_arr.id;
    arrangement.add_track(char_arr);

    let tension_track = Track::new("Narrative Tension", TrackKind::Effect);
    let tension_id    = tension_track.id;
    arrangement.add_track(tension_track);

    // Place clips on the timeline
    let mut clip1 = Clip::new(capsule_a, "Marcus enters café");
    clip1.duration = Some(4.0);
    arrangement.place_clip(char_arr_id, clip1, 0.0);

    let mut clip2 = Clip::new(capsule_b, "Marcus sees her");
    clip2.duration = Some(4.0);
    arrangement.place_clip(char_arr_id, clip2, 4.0);

    // Automation — tension rises from 0 → 0.8 over 8 beats
    if let Some(t) = arrangement.tracks.iter_mut().find(|t| t.track.id == tension_id) {
        let lane = t.add_automation("tension");
        lane.add_point(0.0, 0.0);
        lane.add_point(4.0, 0.3);
        lane.add_point(8.0, 0.8);
    }

    // ── 4. Mixer ──────────────────────────────────────────────────────────────
    let mut mixer = Mixer::new();
    let ch_char = mixer.add_channel("Marcus",    TrackKind::Character);
    let ch_emo  = mixer.add_channel("Sorrow",    TrackKind::Emotion);
    let ch_env  = mixer.add_channel("Café",      TrackKind::Environment);

    println!("🎚️  Mixer:  {} channels + master", mixer.channels.len());

    // ── 5. Trigger scene 0 ───────────────────────────────────────────────────
    println!("\n🎬  Triggering Scene 0: \"Opening — Arrival\"\n");
    session.trigger_scene(0);
    session.commit_queued(); // instant commit for demo

    // ── 6. Tick loop — 2 bars at 120 BPM, 30fps ──────────────────────────────
    //   120 BPM = 2 beats/sec → 8 beats in 4 sec → 120 frames at 30fps
    let sample_rate = 30.0_f64; // 30 "frames" per second for readability
    let frames      = 120usize; // 4 seconds = 2 bars
    let mut packets: Vec<myth_wire::WirePacket> = Vec::new();

    let beats_per_frame = transport.bpm / 60.0 / sample_rate;
    let quantize_beats  = 1.0_f64; // emit automation every beat

    let mut last_beat_reported = -1.0_f64;

    for frame in 0..frames {
        let beat = transport.tick(sample_rate);
        let tick = transport.frame;

        // Transport tick packet — every frame
        let tp = TransportTick::from_transport(&transport);
        packets.push(tp.to_packet(tick));

        // Print beat boundaries
        let (bar, beat_num) = transport.bar_beat();
        if beat.floor() > last_beat_reported {
            last_beat_reported = beat.floor();

            // Automation value at this beat
            if let Some(arr_track) = arrangement.tracks.iter().find(|t| t.track.id == tension_id) {
                for lane in &arr_track.automation {
                    if lane.enabled {
                        let val = lane.value_at(beat);
                        let av  = AutomationValue {
                            track_id: tension_id,
                            param:    lane.param.clone(),
                            value:    val,
                        };
                        let pkt = av.to_packet(tick);
                        packets.push(pkt);

                        // Draw a little tension meter
                        let bars = (val * 20.0) as usize;
                        let bar_str = "█".repeat(bars) + &"░".repeat(20 - bars);
                        println!("  {bar:02}.{beat_num}  tension [{bar_str}] {:.2}", val);
                    }
                }
            }

            // Active clips at this beat
            let active = arrangement.active_clips_at(beat);
            for (track, clip) in &active {
                let ml = MixerLevel {
                    channel_id: ch_char,
                    level:      mixer.effective_level(ch_char),
                };
                packets.push(ml.to_packet(tick));
            }
        }
    }

    // ── 7. Summary ────────────────────────────────────────────────────────────
    println!("\n══════════════════════════════════════════════════");
    println!("  {} total WirePackets emitted over 2 bars", packets.len());

    let tmp_count = packets.iter().filter(|p| p.wire_type == myth_wire::WireType::Temporal).count();
    let dat_count = packets.iter().filter(|p| p.wire_type == myth_wire::WireType::Data).count();
    let ctl_count = packets.iter().filter(|p| p.wire_type == myth_wire::WireType::Control).count();

    println!("  TMP (transport ticks) : {tmp_count}");
    println!("  DAT (automation vals) : {dat_count}");
    println!("  CTL (mixer levels)    : {ctl_count}");

    println!("\n  Final position : {}", transport.position_display());
    println!("  Final BPM      : {:.1}", transport.bpm);
    println!("══════════════════════════════════════════════════");
    println!("\n✓  Theater inbox would have received all of the above.");
    println!("  Next step: wire Transport::tick() into myth-core's clock loop\n");
}
