pub mod logic;
mod theme;

use crate::{
    fyrox::{
        core::{color::Color, pool::Handle},
        engine::ApplicationLoopController,
        gui::{
            border::{Border, BorderBuilder},
            menu::{MenuItem, MenuItemMessage},
            message::{MouseButton, UiMessage},
            scroll_viewer::ScrollViewerBuilder,
            stack_panel::StackPanelBuilder,
            text::{Text, TextBuilder},
            widget::{WidgetBuilder, WidgetMessage},
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode,
            VerticalAlignment,
            grid::{Column, GridBuilder, Row},
        },
    },
    menu::create_menu_item,
    plugin::EditorPlugin,
    Editor,
};
use fyrox::core::uuid::{uuid, Uuid};
use logic::{AxiomStepSequencer, ResonanceHarmonics};
use std::time::Instant;
use theme::*;

pub struct EidolonRackPlugin {
    window: Handle<Window>,
    open_menu_item: Handle<MenuItem>,
    sequencer: AxiomStepSequencer,
    harmonics: ResonanceHarmonics,
    seq_buttons: [[Handle<Border>; 16]; 8],
    harmonic_bars: [Handle<Border>; 8],
    status_text: Handle<Text>,
    last_tick: Option<Instant>,
}

impl Default for EidolonRackPlugin {
    fn default() -> Self {
        Self {
            window: Handle::NONE,
            open_menu_item: Handle::NONE,
            sequencer: AxiomStepSequencer::default(),
            harmonics: ResonanceHarmonics::default(),
            seq_buttons: [[Handle::<Border>::NONE; 16]; 8],
            harmonic_bars: [Handle::<Border>::NONE; 8],
            status_text: Handle::NONE,
            last_tick: None,
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_label(ctx: &mut BuildContext, text: &str, color: Color) -> Handle<Text> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_foreground(brush(color))
            .with_margin(Thickness::uniform(2.0)),
    )
    .with_text(text)
    .build(ctx)
}

fn make_small_label(ctx: &mut BuildContext, text: &str, color: Color) -> Handle<Text> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_foreground(brush(color))
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_text(text)
    .build(ctx)
}

fn make_screw(ctx: &mut BuildContext) -> Handle<Border> {
    BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(8.0)
            .with_height(8.0)
            .with_margin(Thickness::uniform(4.0))
            .with_background(brush(PANEL_BORDER))
            .with_foreground(brush(with_alpha(PANEL_BORDER, 0x88))),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .with_corner_radius(4.0_f32.into())
    .build(ctx)
}

fn make_jack(ctx: &mut BuildContext, color: Color) -> Handle<Border> {
    BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(18.0)
            .with_height(18.0)
            .with_margin(Thickness::uniform(3.0))
            .with_background(brush(BG))
            .with_foreground(brush(color)),
    )
    .with_stroke_thickness(Thickness::uniform(2.0).into())
    .with_corner_radius(9.0_f32.into())
    .build(ctx)
}

fn make_module_shell(
    ctx: &mut BuildContext,
    height: f32,
    border_color: Color,
    content: Handle<UiNode>,
) -> Handle<Border> {
    BorderBuilder::new(
        WidgetBuilder::new()
            .with_height(height)
            .with_margin(Thickness { left: 0.0, right: 0.0, top: 2.0, bottom: 2.0 })
            .with_background(brush(PANEL_DARK))
            .with_foreground(brush(border_color))
            .with_child(content),
    )
    .with_stroke_thickness(Thickness::uniform(1.5).into())
    .build(ctx)
}

fn make_drawbar(
    ctx: &mut BuildContext,
    fill: f32,
    bar_color: Color,
) -> (Handle<Border>, Handle<Border>) {
    let fill_h = (fill * 88.0).max(4.0);
    let fill_bar = BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(8.0)
            .with_height(fill_h)
            .with_vertical_alignment(VerticalAlignment::Bottom)
            .with_background(brush(bar_color))
            .with_foreground(brush(with_alpha(bar_color, 0x00))),
    )
    .build(ctx);
    let track = BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(12.0)
            .with_height(96.0)
            .with_margin(Thickness { left: 4.0, right: 4.0, top: 4.0, bottom: 4.0 })
            .with_background(brush(with_alpha(BG, 0xaa)))
            .with_foreground(brush(with_alpha(PANEL_BORDER, 0x88)))
            .with_child(fill_bar),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .with_corner_radius(4.0_f32.into())
    .build(ctx);
    (track, fill_bar)
}

// ── Module builders ───────────────────────────────────────────────────────────

fn build_mythos_cartographer(ctx: &mut BuildContext) -> Handle<UiNode> {
    let led = BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(36.0)
            .with_height(36.0)
            .with_margin(Thickness::uniform(4.0))
            .with_background(brush(with_alpha(SECONDARY, 0x33)))
            .with_foreground(brush(with_alpha(SECONDARY, 0x88))),
    )
    .with_stroke_thickness(Thickness::uniform(2.0).into())
    .with_corner_radius(18.0_f32.into())
    .build(ctx);
    let led_panel = BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(64.0)
            .with_background(brush(PANEL_DARK))
            .with_foreground(brush(PANEL_BORDER))
            .with_child(led),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .build(ctx);

    let title = make_small_label(ctx, "MYTHOS CARTOGRAPHER // 7.34.1", with_alpha(PRIMARY, 0x99));
    let screen_inner = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(title),
    )
    .build(ctx);
    let screen = BorderBuilder::new(
        WidgetBuilder::new()
            .with_background(brush(PANEL_LIGHT))
            .with_foreground(brush(with_alpha(PRIMARY, 0x80)))
            .with_child(screen_inner),
    )
    .with_stroke_thickness(Thickness::uniform(2.0).into())
    .build(ctx);

    let s1 = make_small_label(ctx, "SYSTEM STATUS", with_alpha(PRIMARY, 0x88));
    let s2 = make_small_label(ctx, "NARRATIVE FLOW: STABLE", with_alpha(SECONDARY, 0xaa));
    let s3 = make_small_label(ctx, "CAUSALITY: COHERENT", with_alpha(SECONDARY, 0xaa));
    let status_inner = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(s1)
            .with_child(s2)
            .with_child(s3),
    )
    .build(ctx);
    let status = BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(140.0)
            .with_background(brush(PANEL_DARK))
            .with_foreground(brush(PANEL_BORDER))
            .with_child(status_inner),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .build(ctx);

    let grid = GridBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(led_panel)
            .with_child(screen)
            .with_child(status),
    )
    .add_column(Column::strict(64.0))
    .add_column(Column::stretch())
    .add_column(Column::strict(140.0))
    .add_row(Row::stretch())
    .build(ctx);
    ctx[led_panel].set_column(0); ctx[led_panel].set_row(0);
    ctx[screen].set_column(1);   ctx[screen].set_row(0);
    ctx[status].set_column(2);   ctx[status].set_row(0);
    grid.transmute()
}

fn build_quantum_director(ctx: &mut BuildContext) -> Handle<UiNode> {
    let io_title = make_small_label(ctx, "ENTANGLEMENT I/O", with_alpha(PRIMARY, 0x88));
    let jacks: Vec<_> = (0..4).map(|_| make_jack(ctx, PANEL_BORDER)).collect();
    let row1 = StackPanelBuilder::new(
        WidgetBuilder::new().with_child(jacks[0]).with_child(jacks[1]),
    )
    .with_orientation(Orientation::Horizontal)
    .build(ctx);
    let row2 = StackPanelBuilder::new(
        WidgetBuilder::new().with_child(jacks[2]).with_child(jacks[3]),
    )
    .with_orientation(Orientation::Horizontal)
    .build(ctx);
    let patch_inner = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(io_title)
            .with_child(row1)
            .with_child(row2),
    )
    .build(ctx);
    let patch_bay = BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(96.0)
            .with_background(brush(PANEL_DARK))
            .with_foreground(brush(PANEL_BORDER))
            .with_child(patch_inner),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .build(ctx);

    let orrery_lbl = make_small_label(ctx, "ENTANGLEMENT ORRERY", with_alpha(PRIMARY, 0x77));
    let orrery_inner = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(orrery_lbl),
    )
    .build(ctx);
    let orrery = BorderBuilder::new(
        WidgetBuilder::new()
            .with_background(brush(BG))
            .with_foreground(brush(with_alpha(PRIMARY, 0xcc)))
            .with_child(orrery_inner),
    )
    .with_stroke_thickness(Thickness::uniform(2.0).into())
    .build(ctx);

    let d1 = make_small_label(ctx, "SIM DIRECTOR", with_alpha(PRIMARY, 0x88));
    let d2 = make_small_label(ctx, "STATE: ACTIVE", with_alpha(SECONDARY, 0xaa));
    let d3 = make_small_label(ctx, "OVERRIDE: OFF", with_alpha(SECONDARY, 0x77));
    let dir_inner = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(d1)
            .with_child(d2)
            .with_child(d3),
    )
    .build(ctx);
    let director = BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(140.0)
            .with_background(brush(PANEL_DARK))
            .with_foreground(brush(PANEL_BORDER))
            .with_child(dir_inner),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .build(ctx);

    let grid = GridBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(patch_bay)
            .with_child(orrery)
            .with_child(director),
    )
    .add_column(Column::strict(96.0))
    .add_column(Column::stretch())
    .add_column(Column::strict(140.0))
    .add_row(Row::stretch())
    .build(ctx);
    ctx[patch_bay].set_column(0); ctx[patch_bay].set_row(0);
    ctx[orrery].set_column(1);   ctx[orrery].set_row(0);
    ctx[director].set_column(2); ctx[director].set_row(0);
    grid.transmute()
}

fn build_flux_spectrogram(ctx: &mut BuildContext) -> Handle<UiNode> {
    let wave: [f32; 20] = [
        0.50, 0.30, 0.70, 0.40, 0.60, 0.20, 0.80, 0.50, 0.56, 0.44,
        0.50, 0.36, 0.64, 0.50, 0.90, 0.10, 0.50, 0.70, 0.30, 0.40,
    ];
    let mut bars_wb = WidgetBuilder::new()
        .with_margin(Thickness::uniform(6.0))
        .with_vertical_alignment(VerticalAlignment::Bottom);
    for (i, &amp) in wave.iter().enumerate() {
        let bar_h = (amp * 70.0).max(4.0);
        let color = if i < 10 { SECONDARY } else { PRIMARY };
        let bar = BorderBuilder::new(
            WidgetBuilder::new()
                .with_width(10.0)
                .with_height(bar_h)
                .with_margin(Thickness { left: 2.0, right: 2.0, top: 0.0, bottom: 0.0 })
                .with_vertical_alignment(VerticalAlignment::Bottom)
                .with_background(brush(color))
                .with_foreground(brush(with_alpha(color, 0x00))),
        )
        .build(ctx);
        bars_wb = bars_wb.with_child(bar);
    }
    let bars = StackPanelBuilder::new(bars_wb)
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

    BorderBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_background(brush(BG))
            .with_foreground(brush(with_alpha(SECONDARY, 0x55)))
            .with_child(bars),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .build(ctx)
    .transmute()
}

fn build_axiom_sequencer(
    ctx: &mut BuildContext,
    sequencer: &AxiomStepSequencer,
    harmonics: &ResonanceHarmonics,
) -> (Handle<UiNode>, [[Handle<Border>; 16]; 8], [Handle<Border>; 8]) {
    let mut seq_buttons: [[Handle<Border>; 16]; 8] = [[Handle::<Border>::NONE; 16]; 8];
    let mut rows: Vec<Handle<UiNode>> = Vec::new();

    for law_idx in 0..8 {
        let row_color = match law_idx {
            0..=2 => with_alpha(PRIMARY, 0x80),
            3..=5 => with_alpha(SECONDARY, 0x80),
            _ => with_alpha(ACCENT, 0x80),
        };
        let law_label = make_small_label(ctx, sequencer.laws[law_idx], with_alpha(TEXT_GRAY, 0x99));
        let mut row_wb = WidgetBuilder::new()
            .with_margin(Thickness { left: 2.0, right: 2.0, top: 1.0, bottom: 1.0 })
            .with_child(law_label);

        for step_idx in 0..16 {
            let active = sequencer.steps[law_idx][step_idx];
            let (bg, fg) = if active {
                let c = match law_idx {
                    0..=2 => PRIMARY,
                    3..=5 => SECONDARY,
                    _ => ACCENT,
                };
                (brush(with_alpha(c, 0xdd)), brush(with_alpha(c, 0xff)))
            } else {
                (brush(PANEL_DARK), brush(row_color))
            };
            let btn = BorderBuilder::new(
                WidgetBuilder::new()
                    .with_width(18.0)
                    .with_height(18.0)
                    .with_margin(Thickness::uniform(1.5))
                    .with_background(bg)
                    .with_foreground(fg),
            )
            .with_stroke_thickness(Thickness::uniform(1.5).into())
            .with_corner_radius(9.0_f32.into())
            .build(ctx);
            seq_buttons[law_idx][step_idx] = btn;
            row_wb = row_wb.with_child(btn);
        }

        let row = StackPanelBuilder::new(row_wb)
            .with_orientation(Orientation::Horizontal)
            .build(ctx);
        rows.push(row.transmute());
    }

    let mut seq_wb = WidgetBuilder::new().with_margin(Thickness::uniform(4.0));
    for r in rows {
        seq_wb = seq_wb.with_child(r);
    }
    let seq_panel = StackPanelBuilder::new(seq_wb).build(ctx);
    let seq_border = BorderBuilder::new(
        WidgetBuilder::new()
            .with_background(brush(PANEL_DARK))
            .with_foreground(brush(PANEL_BORDER))
            .with_child(seq_panel),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .build(ctx);

    // Drawbars
    let bar_colors = [SECONDARY, SECONDARY, SECONDARY, ACCENT, ACCENT, PINK, PINK, PINK];
    let mut harmonic_bars: [Handle<Border>; 8] = [Handle::<Border>::NONE; 8];
    let mut tracks_wb = WidgetBuilder::new();
    for i in 0..8 {
        let (track, fill_bar) = make_drawbar(ctx, harmonics.values[i], bar_colors[i]);
        harmonic_bars[i] = fill_bar;
        tracks_wb = tracks_wb.with_child(track);
    }
    let tracks_row = StackPanelBuilder::new(tracks_wb)
        .with_orientation(Orientation::Horizontal)
        .build(ctx);
    let res_label = make_small_label(ctx, "RESONANCE HARMONICS", with_alpha(PRIMARY, 0x88));
    let drawbar_inner = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(res_label)
            .with_child(tracks_row),
    )
    .build(ctx);
    let drawbar_panel = BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(160.0)
            .with_background(brush(PANEL_DARK))
            .with_foreground(brush(PANEL_BORDER))
            .with_child(drawbar_inner),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .build(ctx);

    let grid = GridBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(seq_border)
            .with_child(drawbar_panel),
    )
    .add_column(Column::stretch())
    .add_column(Column::strict(160.0))
    .add_row(Row::stretch())
    .build(ctx);
    ctx[seq_border].set_column(0);    ctx[seq_border].set_row(0);
    ctx[drawbar_panel].set_column(1); ctx[drawbar_panel].set_row(0);

    (grid.transmute(), seq_buttons, harmonic_bars)
}

fn build_causality_sequencer(ctx: &mut BuildContext) -> Handle<UiNode> {
    let lt = make_small_label(ctx, "EVENT HORIZON I/O", with_alpha(PRIMARY, 0x88));
    let lj: Vec<_> = (0..4).map(|_| make_jack(ctx, PANEL_BORDER)).collect();
    let lr1 = StackPanelBuilder::new(
        WidgetBuilder::new().with_child(lj[0]).with_child(lj[1]),
    )
    .with_orientation(Orientation::Horizontal)
    .build(ctx);
    let lr2 = StackPanelBuilder::new(
        WidgetBuilder::new().with_child(lj[2]).with_child(lj[3]),
    )
    .with_orientation(Orientation::Horizontal)
    .build(ctx);
    let left_inner = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(lt)
            .with_child(lr1)
            .with_child(lr2),
    )
    .build(ctx);
    let left_io = BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(90.0)
            .with_background(brush(PANEL_DARK))
            .with_foreground(brush(PANEL_BORDER))
            .with_child(left_inner),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .build(ctx);

    let symbols = [
        "⏃","⎍","⎎","⏁","⎏","⎑","⎓","⎙","⎛","⎝",
        "⎟","⎡","⎣","⎥","⎧","⎩","⎫","⎭","⎯","⎱",
        "⎳","⎵","⎷","⎹","⎻","⎽","⎾","⏄","⏆","⏈",
        "⏊","⏌","⏎","⏐","⏒","⏔","⏖","⏘","⏚","⏛",
    ];
    let mut sym_wb = WidgetBuilder::new().with_margin(Thickness::uniform(4.0));
    for sym in &symbols {
        sym_wb = sym_wb.with_child(make_small_label(ctx, sym, with_alpha(SECONDARY, 0xcc)));
    }
    let sym_row = StackPanelBuilder::new(sym_wb)
        .with_orientation(Orientation::Horizontal)
        .build(ctx);
    let sym_grid = BorderBuilder::new(
        WidgetBuilder::new()
            .with_background(brush(BG))
            .with_foreground(brush(with_alpha(SECONDARY, 0x44)))
            .with_child(sym_row),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .build(ctx);

    let rt = make_small_label(ctx, "CHRONON I/O", with_alpha(PRIMARY, 0x88));
    let rj: Vec<_> = (0..4).map(|i| {
        make_jack(ctx, if i < 2 { SECONDARY } else { PANEL_BORDER })
    })
    .collect();
    let rr1 = StackPanelBuilder::new(
        WidgetBuilder::new().with_child(rj[0]).with_child(rj[1]),
    )
    .with_orientation(Orientation::Horizontal)
    .build(ctx);
    let rr2 = StackPanelBuilder::new(
        WidgetBuilder::new().with_child(rj[2]).with_child(rj[3]),
    )
    .with_orientation(Orientation::Horizontal)
    .build(ctx);
    let right_inner = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(rt)
            .with_child(rr1)
            .with_child(rr2),
    )
    .build(ctx);
    let right_io = BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(90.0)
            .with_background(brush(PANEL_DARK))
            .with_foreground(brush(PANEL_BORDER))
            .with_child(right_inner),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .build(ctx);

    let grid = GridBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(left_io)
            .with_child(sym_grid)
            .with_child(right_io),
    )
    .add_column(Column::strict(90.0))
    .add_column(Column::stretch())
    .add_column(Column::strict(90.0))
    .add_row(Row::stretch())
    .build(ctx);
    ctx[left_io].set_column(0);  ctx[left_io].set_row(0);
    ctx[sym_grid].set_column(1); ctx[sym_grid].set_row(0);
    ctx[right_io].set_column(2); ctx[right_io].set_row(0);
    grid.transmute()
}

fn build_resonance_tuner(ctx: &mut BuildContext) -> Handle<UiNode> {
    let tt = make_small_label(ctx, "TOPOLOGICAL INVERSION", with_alpha(PRIMARY, 0x88));
    let tl1 = make_small_label(ctx, "   /\\   /\\", with_alpha(SECONDARY, 0xcc));
    let tl2 = make_small_label(ctx, "  /  \\ /  \\", with_alpha(SECONDARY, 0x99));
    let tl3 = make_small_label(ctx, " o   *   *  o", with_alpha(PRIMARY, 0xdd));
    let topo_inner = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(tt)
            .with_child(tl1)
            .with_child(tl2)
            .with_child(tl3),
    )
    .build(ctx);
    let topo = BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(160.0)
            .with_background(brush(PANEL_DARK))
            .with_foreground(brush(PANEL_BORDER))
            .with_child(topo_inner),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .build(ctx);

    // Knob placeholders
    let make_knob = |ctx: &mut BuildContext| {
        let nub = BorderBuilder::new(
            WidgetBuilder::new()
                .with_width(4.0)
                .with_height(14.0)
                .with_vertical_alignment(VerticalAlignment::Top)
                .with_horizontal_alignment(HorizontalAlignment::Center)
                .with_background(brush(PRIMARY))
                .with_foreground(brush(with_alpha(PRIMARY, 0x00))),
        )
        .build(ctx);
        BorderBuilder::new(
            WidgetBuilder::new()
                .with_width(44.0)
                .with_height(44.0)
                .with_margin(Thickness::uniform(4.0))
                .with_background(brush(PANEL_LIGHT))
                .with_foreground(brush(PANEL_BORDER))
                .with_child(nub),
        )
        .with_stroke_thickness(Thickness::uniform(2.0).into())
        .with_corner_radius(22.0_f32.into())
        .build(ctx)
    };
    let k1 = make_knob(ctx);
    let k2 = make_knob(ctx);
    let k3 = make_knob(ctx);
    let knobs_row = StackPanelBuilder::new(
        WidgetBuilder::new().with_child(k1).with_child(k2).with_child(k3),
    )
    .with_orientation(Orientation::Horizontal)
    .build(ctx);

    let d1 = make_small_label(ctx, "    \u{25c8}    ", with_alpha(ACCENT, 0xdd));
    let d2 = make_small_label(ctx, "   / \\   ", with_alpha(SECONDARY, 0xaa));
    let d3 = make_small_label(ctx, "  /   \\  ", with_alpha(SECONDARY, 0x88));
    let d4 = make_small_label(ctx, "  \\   /  ", with_alpha(SECONDARY, 0x88));
    let d5 = make_small_label(ctx, "   \\ /   ", with_alpha(SECONDARY, 0xaa));
    let d6 = make_small_label(ctx, "    \u{25c8}    ", with_alpha(ACCENT, 0xdd));
    let diamond = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_horizontal_alignment(HorizontalAlignment::Center)
            .with_child(d1).with_child(d2).with_child(d3)
            .with_child(d4).with_child(d5).with_child(d6),
    )
    .build(ctx);

    let knob_panel_inner = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(knobs_row)
            .with_child(diamond),
    )
    .build(ctx);
    let knob_panel = BorderBuilder::new(
        WidgetBuilder::new()
            .with_background(brush(PANEL_DARK))
            .with_foreground(brush(PANEL_BORDER))
            .with_child(knob_panel_inner),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .build(ctx);

    let readout_val = make_label(ctx, "E:8.03", ACCENT);
    let readout_lbl = make_small_label(ctx, "RESONANCE", with_alpha(PRIMARY, 0x77));
    let readout_inner = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(readout_val)
            .with_child(readout_lbl),
    )
    .build(ctx);
    let readout = BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(80.0)
            .with_background(brush(PANEL_DARK))
            .with_foreground(brush(PANEL_BORDER))
            .with_child(readout_inner),
    )
    .with_stroke_thickness(Thickness::uniform(1.0).into())
    .build(ctx);

    let grid = GridBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_child(topo)
            .with_child(knob_panel)
            .with_child(readout),
    )
    .add_column(Column::strict(160.0))
    .add_column(Column::stretch())
    .add_column(Column::strict(80.0))
    .add_row(Row::stretch())
    .build(ctx);
    ctx[topo].set_column(0);       ctx[topo].set_row(0);
    ctx[knob_panel].set_column(1); ctx[knob_panel].set_row(0);
    ctx[readout].set_column(2);    ctx[readout].set_row(0);
    grid.transmute()
}

fn build_rack_rail(ctx: &mut BuildContext) -> Handle<UiNode> {
    let mut rail_wb = WidgetBuilder::new()
        .with_width(28.0)
        .with_background(brush(PANEL_DARK))
        .with_foreground(brush(PANEL_BORDER));
    for _ in 0..14 {
        rail_wb = rail_wb.with_child(make_screw(ctx));
    }
    BorderBuilder::new(rail_wb)
        .with_stroke_thickness(Thickness::uniform(1.0).into())
        .build(ctx)
        .transmute()
}

// ── Plugin impl ───────────────────────────────────────────────────────────────

impl EidolonRackPlugin {
    pub const OPEN_EIDOLON_RACK: Uuid = uuid!("b3c4d5e6-f7a8-9b0c-d1e2-f3a4b5c6d7e8");

    fn build_window(&mut self, ctx: &mut BuildContext) -> Handle<Window> {
        let m1 = build_mythos_cartographer(ctx);
        let module1 = make_module_shell(ctx, 110.0, with_alpha(PRIMARY, 0x55), m1);

        let m2 = build_quantum_director(ctx);
        let module2 = make_module_shell(ctx, 110.0, with_alpha(SECONDARY, 0x55), m2);

        let m3 = build_flux_spectrogram(ctx);
        let module3 = make_module_shell(ctx, 100.0, with_alpha(SECONDARY, 0x44), m3);

        let (m4, seq_btns, harm_bars) =
            build_axiom_sequencer(ctx, &self.sequencer, &self.harmonics);
        self.seq_buttons = seq_btns;
        self.harmonic_bars = harm_bars;
        let module4 = make_module_shell(ctx, 210.0, with_alpha(PRIMARY, 0x44), m4);

        let m5 = build_causality_sequencer(ctx);
        let module5 = make_module_shell(ctx, 140.0, with_alpha(ACCENT, 0x44), m5);

        let m6 = build_resonance_tuner(ctx);
        let module6 = make_module_shell(ctx, 140.0, with_alpha(ACCENT, 0x55), m6);

        let status = TextBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .on_column(0)
                .with_height(22.0)
                .with_margin(Thickness { left: 8.0, right: 8.0, top: 2.0, bottom: 2.0 })
                .with_foreground(brush(with_alpha(PRIMARY, 0xaa))),
        )
        .with_text("EIDOLON SYNTHESIS RACK  //  RESONANCE: 432 Hz  //  LATTICE: NOMINAL")
        .build(ctx);
        self.status_text = status;

        let stack = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(module1)
                .with_child(module2)
                .with_child(module3)
                .with_child(module4)
                .with_child(module5)
                .with_child(module6),
        )
        .build(ctx);

        let scroll = ScrollViewerBuilder::new(
            WidgetBuilder::new()
                .with_background(brush(PANEL_DARK))
                .with_foreground(brush(PANEL_BORDER)),
        )
        .with_content(stack)
        .build(ctx);

        let left_rail = build_rack_rail(ctx);
        let right_rail = build_rack_rail(ctx);

        let rack_grid = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .on_column(0)
                .with_background(brush(BG))
                .with_child(left_rail)
                .with_child(scroll)
                .with_child(right_rail),
        )
        .add_column(Column::strict(28.0))
        .add_column(Column::stretch())
        .add_column(Column::strict(28.0))
        .add_row(Row::stretch())
        .build(ctx);

        ctx[left_rail].set_column(0);  ctx[left_rail].set_row(0);
        ctx[scroll].set_column(1);     ctx[scroll].set_row(0);
        ctx[right_rail].set_column(2); ctx[right_rail].set_row(0);

        let outer = GridBuilder::new(
            WidgetBuilder::new()
                .with_background(brush(BG))
                .with_child(rack_grid)
                .with_child(status),
        )
        .add_column(Column::stretch())
        .add_row(Row::stretch())
        .add_row(Row::strict(26.0))
        .build(ctx);

        WindowBuilder::new(
            WidgetBuilder::new()
                .with_width(760.0)
                .with_height(840.0)
                .with_background(brush(BG)),
        )
        .with_title(WindowTitle::text("EIDOLON SYNTHESIS RACK"))
        .with_content(outer)
        .build(ctx)
    }
}

impl EditorPlugin for EidolonRackPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let ctx = &mut ui.build_ctx();
        self.open_menu_item = create_menu_item(
            "Eidolon Synthesis Rack",
            Self::OPEN_EIDOLON_RACK,
            vec![],
            ctx,
        );
        ui.send(
            editor.menu.utils_menu.menu,
            MenuItemMessage::AddItem(self.open_menu_item),
        );
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.open_menu_item && self.window.is_none() {
                let ui = editor.engine.user_interfaces.first_mut();
                let ctx = &mut ui.build_ctx();
                self.window = self.build_window(ctx);
                self.last_tick = Some(Instant::now());
                ui.send(
                    self.window,
                    WindowMessage::Open {
                        alignment: WindowAlignment::Center,
                        modal: false,
                        focus_content: true,
                    },
                );
            }
        }

        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                self.window = Handle::NONE;
                self.last_tick = None;
            }
        }

        // Sequencer button toggle on left-click
        if let Some(WidgetMessage::MouseDown { button: MouseButton::Left, .. }) = message.data() {
            'find_btn: for law in 0..8 {
                for step in 0..16 {
                    if message.destination() == self.seq_buttons[law][step] {
                        self.sequencer.toggle_step(law, step);
                        let active = self.sequencer.steps[law][step];
                        let step_is_current = step == self.sequencer.current_step;
                        let ui = editor.engine.user_interfaces.first_mut();
                        Self::refresh_button(ui, self.seq_buttons[law][step], active, step_is_current, law);
                        break 'find_btn;
                    }
                }
            }
        }
    }

    fn on_update(&mut self, editor: &mut Editor, _loop_controller: ApplicationLoopController) {
        if self.window.is_none() { return; }

        let step_duration = std::time::Duration::from_secs_f32(
            60.0 / (self.sequencer.bpm * 4.0), // 16th notes
        );

        let now = Instant::now();
        let should_tick = self.last_tick
            .map(|t| now.duration_since(t) >= step_duration)
            .unwrap_or(false);

        if should_tick {
            let prev_step = self.sequencer.current_step;
            let active_laws: Vec<String> = self.sequencer.tick().iter().map(|s| s.to_string()).collect();
            let curr_step = self.sequencer.current_step;
            self.last_tick = Some(now);

            let ui = editor.engine.user_interfaces.first_mut();

            // Refresh previous step column (remove playhead glow)
            for law in 0..8 {
                let active = self.sequencer.steps[law][prev_step];
                Self::refresh_button(ui, self.seq_buttons[law][prev_step], active, false, law);
            }

            // Refresh current step column (add playhead glow)
            for law in 0..8 {
                let active = self.sequencer.steps[law][curr_step];
                Self::refresh_button(ui, self.seq_buttons[law][curr_step], active, true, law);
            }

            // Update status text with firing laws
            if !active_laws.is_empty() {
                let msg = format!(
                    "STEP {:02}  //  FIRING: {}  //  BPM: {}",
                    curr_step,
                    active_laws.join(" · "),
                    self.sequencer.bpm as u32,
                );
                ui.send(self.status_text, crate::fyrox::gui::text::TextMessage::Text(msg));
            }
        }
    }
}

impl EidolonRackPlugin {
    fn row_color(law: usize) -> Color {
        match law {
            0..=2 => with_alpha(PRIMARY, 0x80),
            3..=5 => with_alpha(SECONDARY, 0x80),
            6 => with_alpha(ACCENT, 0x80),
            _ => with_alpha(PINK, 0x80),
        }
    }

    fn refresh_button(
        ui: &mut crate::fyrox::gui::UserInterface,
        btn: Handle<Border>,
        active: bool,
        playhead: bool,
        law: usize,
    ) {
        let row_col = Self::row_color(law);
        let bg = if active {
            brush(with_alpha(row_col, 0xdd))
        } else {
            brush(with_alpha(PANEL_DARK, 0xaa))
        };
        let fg = if playhead {
            brush(with_alpha(PRIMARY, 0xff))
        } else {
            brush(with_alpha(row_col, 0x88))
        };
        ui.send(btn, WidgetMessage::Background(bg));
        ui.send(btn, WidgetMessage::Foreground(fg));
    }
}
