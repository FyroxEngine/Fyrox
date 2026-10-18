// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.

//! Generate isolated scenes for testing native IME input in the editor and in UI preview mode.
use fyrox::{
    core::{algebra::Vector2, visitor::Visitor},
    event_loop::EventLoop,
    gui::{
        Thickness, UserInterface,
        canvas::CanvasBuilder,
        formatted_text::WrapMode,
        text::TextBuilder,
        text_box::{TextBoxBuilder, TextCommitMode},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
    },
    scene::{Scene, base::BaseBuilder, pivot::PivotBuilder},
};
use fyroxed_base::{Editor, StartupData};

use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Expected project directory")?,
    );
    let data = root.join("data");
    fs::create_dir_all(&data)?;
    let scene_path = data.join("inspector.rgs");
    if !scene_path.exists() {
        let mut scene = Scene::new();
        for name in [
            "中文节点 - 修改 Inspector Name",
            "Second node - 切换焦点测试",
        ] {
            PivotBuilder::new(BaseBuilder::new().with_name(name)).build(&mut scene.graph);
        }
        let mut visitor = Visitor::new();
        scene.save("Scene", &mut visitor)?;
        visitor.save_ascii_to_file(&scene_path)?;
        println!("Created {}", scene_path.display());
    }

    let ui_path = data.join("text-input.ui");
    if !ui_path.exists() {
        let mut ui = UserInterface::new(Vector2::new(900.0, 600.0));
        let ctx = &mut ui.build_ctx();
        TextBuilder::new(WidgetBuilder::new()
            .with_width(850.0).with_height(75.0)
            .with_desired_position(Vector2::new(20.0, 15.0)))
            .with_text("IME 中文输入测试：在 Inspector 中编辑文本框的 Name / Text 属性。\n运行 scripts/test-ime.ps1 可在编辑器内打开交互测试窗口。")
            .build(ctx);
        for (index, (name, text, multiline, editable, height)) in [
            (
                "Single line",
                "单行输入：请选择我并输入中文",
                false,
                true,
                38.0,
            ),
            (
                "Multiline",
                "多行输入：第一行\n第二行，检查候选窗位置",
                true,
                true,
                130.0,
            ),
            (
                "Focus switch",
                "第二个输入框：检查焦点切换与选区替换",
                false,
                true,
                38.0,
            ),
            (
                "Read only",
                "只读文本：此处不应开启输入法",
                false,
                false,
                38.0,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let y = [105.0, 165.0, 320.0, 385.0][index];
            TextBoxBuilder::new(
                WidgetBuilder::new()
                    .with_name(name)
                    .with_width(720.0)
                    .with_height(height)
                    .with_desired_position(Vector2::new(25.0, y)),
            )
            .with_text(text)
            .with_padding(Thickness::uniform(6.0))
            .with_multiline(multiline)
            .with_editable(editable)
            .with_wrap(if multiline {
                WrapMode::Word
            } else {
                WrapMode::NoWrap
            })
            .with_text_commit_mode(TextCommitMode::Immediate)
            .build(ctx);
        }
        TextBuilder::new(WidgetBuilder::new().with_width(850.0).with_height(70.0)
            .with_desired_position(Vector2::new(20.0, 455.0)))
            .with_text("Inspector 可直接测试 Name / Text 属性，搜索框可测试即时更新。\n保存 .ui 后重新打开，检查已确认的中文是否保留。")
            .build(ctx);
        ui.save(&ui_path)?;
        println!("Created {}", ui_path.display());
    }
    let mode = std::env::args().nth(2).unwrap_or_default();
    if mode == "--editor" {
        let scene = std::env::args()
            .nth(3)
            .unwrap_or_else(|| "data/inspector.rgs".into());
        let mut editor = Editor::new(Some(StartupData {
            working_directory: root,
            scenes: vec![scene.into()],
            named_objects: false,
        }));
        let ctx = &mut editor.engine.user_interfaces.first_mut().build_ctx();
        let mut children = Vec::new();
        children.push(TextBuilder::new(WidgetBuilder::new().with_width(620.0).with_height(50.0)
            .with_desired_position(Vector2::new(15.0, 8.0)))
            .with_text("单行 / 多行中文输入测试；关闭此窗口可测试右侧 Inspector。\n拼音下划线与候选窗应跟随光标，Esc 取消组合，不修改原文。")
            .build(ctx).to_base());
        for (index, (text, editable, multiline, height)) in [
            ("单行：请选中后输入中文", true, false, 36.0),
            ("多行输入第一行\n第二行：在这里输入中文", true, true, 100.0),
            ("另一个输入框：测试焦点切换", true, false, 36.0),
            ("只读：IME 应在这里停用", false, false, 36.0),
        ]
        .into_iter()
        .enumerate()
        {
            let field = TextBoxBuilder::new(
                WidgetBuilder::new()
                    .with_width(610.0)
                    .with_height(height)
                    .with_desired_position(Vector2::new(15.0, [68.0, 118.0, 235.0, 285.0][index])),
            )
            .with_text(text)
            .with_editable(editable)
            .with_multiline(multiline)
            .with_padding(Thickness::uniform(5.0))
            .with_text_commit_mode(TextCommitMode::Immediate)
            .build(ctx);
            children.push(field.to_base());
        }
        let content = CanvasBuilder::new(WidgetBuilder::new().with_children(children)).build(ctx);
        WindowBuilder::new(
            WidgetBuilder::new()
                .with_name("IME Test")
                .with_width(660.0)
                .with_height(390.0)
                .with_desired_position(Vector2::new(100.0, 100.0)),
        )
        .with_title(WindowTitle::text("IME 中文输入测试"))
        .with_content(content)
        .build(ctx);
        editor.run(EventLoop::new()?);
    }
    Ok(())
}
