use std::{
    convert::Infallible,
    fmt::{Debug, Display, Write},
    ops::Range,
    str::FromStr,
};

use crate::{
    brush::Brush,
    font::FontResource,
    formatted_text::{FormattedTextBuilder, Run, RunSet},
};

/// A BBCode parser that is specially designed for the formatting options
/// available to [`FormattedText`](crate::formatted_text::FormattedText).
/// The available tags are:
/// * `[b]` **bold text** `[/b]`
/// * `[i]` *italic text* `[/i]`
/// * `[color=red]` red text `[/color]` (can be shortened to `[c=red]`... `[/c]`, and can use hex color as in `[color=#FF0000]`)
/// * `[size=24]` large text `[/size]` (can be shortened to `[s=24]` ... `[/s]`)
/// * `[shadow]` shadowed text `[/shadow]` (can be shortened to `[sh]` ... `[/sh]` and can change shadow color with `[shadow=blue]`)
/// * `[br]` for a line break.
#[derive(Debug, Clone)]
pub struct BBCode {
    /// The plain text without tags.
    pub text: String,
    /// The tags that were removed from the text.
    pub tags: Box<[BBTag]>,
}

#[derive(Clone, Eq, PartialEq)]
pub struct BBTag {
    /// The position of the tag in the plain text, with 0 being the beginning of the text.
    /// The position is relative to the text *without* tags, so for example, if the code text were:
    /// `"Here is [b]bold text[/b]."` then the plain text would be:
    /// `"Here is bold text."` and the tags would be at positions 8 and 17.
    pub position: usize,
    /// The content of the tag.
    pub data: BBTagData,
}

impl std::ops::Deref for BBTag {
    type Target = BBTagData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl Debug for BBTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl Display for BBTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.data, self.position)
    }
}

/// The content of a BBCode tag.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BBTagData {
    /// The text at the beginning of the tag, not including an initial /, upto and not including an = if present.
    /// For example, if the tag were `[size=24]` then the label would be "size".
    /// If the tag were `[/size]` then the label would be "size".
    pub label: String,
    /// The text that follows the = in the tag.
    pub argument: Option<String>,
    /// True if the tag starts with a / to indicate that it is the end of some span.
    pub is_close: bool,
}

impl BBTagData {
    pub fn open(label: String, argument: Option<String>) -> Self {
        Self {
            is_close: false,
            label,
            argument,
        }
    }
    pub fn close(label: String, argument: Option<String>) -> Self {
        Self {
            is_close: true,
            label,
            argument,
        }
    }
}

impl Display for BBTagData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('[')?;
        if self.is_close {
            f.write_char('/')?;
        }
        f.write_str(&self.label)?;
        if let Some(arg) = &self.argument {
            f.write_char('=')?;
            f.write_str(arg)?;
        }
        f.write_char(']')
    }
}

impl FromStr for BBTagData {
    type Err = Infallible;
    fn from_str(source: &str) -> Result<Self, Infallible> {
        let mut source = source.as_bytes();
        let mut is_close = false;
        if let Some((b'/', rest)) = source.split_first() {
            is_close = true;
            source = rest;
        }
        if let Some(equals_pos) = source.iter().position(|c| *c == b'=') {
            let (label, argument) = source.split_at(equals_pos);
            let label = label.trim_ascii();
            let argument = argument[1..].trim_ascii();
            Ok(Self {
                is_close,
                label: std::str::from_utf8(label).unwrap().to_string(),
                argument: Some(std::str::from_utf8(argument).unwrap().to_string()),
            })
        } else {
            Ok(Self {
                is_close,
                label: std::str::from_utf8(source.trim_ascii())
                    .unwrap()
                    .to_string(),
                argument: None,
            })
        }
    }
}

impl FromStr for BBCode {
    type Err = Infallible;
    fn from_str(source: &str) -> Result<Self, Infallible> {
        let mut source = source.as_bytes();
        let mut text = Vec::new();
        let mut tags = Vec::new();
        while !source.is_empty() {
            match source[0] {
                b'[' => {
                    source = &source[1..];
                    if let Some(end_pos) = source.iter().position(|c| *c == b']') {
                        let content = std::str::from_utf8(&source[0..end_pos]).unwrap();
                        source = &source[end_pos + 1..];
                        let data: BBTagData = content.parse()?;
                        if !data.is_close && data.argument.is_none() && data.label == "br" {
                            text.push(b'\n');
                        } else {
                            tags.push(BBTag {
                                position: text.len(),
                                data,
                            });
                        }
                    } else {
                        source = &[];
                    }
                }
                c => {
                    text.push(c);
                    source = &source[1..];
                }
            }
        }
        Ok(Self {
            text: std::str::from_utf8(&text).unwrap().to_string(),
            tags: tags.into_boxed_slice(),
        })
    }
}

fn find_close<'a, I: Iterator<Item = &'a BBTag>>(label: &str, iter: I) -> Option<&'a BBTag> {
    let mut nesting_level = 0;
    for tag in iter {
        if tag.is_close {
            if nesting_level == 0 {
                return (tag.label == label).then_some(tag);
            } else {
                nesting_level -= 1;
            }
        } else {
            nesting_level += 1;
        }
    }
    None
}

fn find_font_run(runs: &mut [Run], pos: u32) -> Option<&mut Run> {
    runs.iter_mut()
        .rev()
        .find(|r| r.range.contains(&pos) && r.font().is_some())
}

fn update_font(
    runs: &mut RunSet,
    range: Range<u32>,
    new_font: Option<&FontResource>,
    other_font: Option<&FontResource>,
    bold_italic: Option<&FontResource>,
) {
    if let Some(run) = find_font_run(runs, range.start) {
        if other_font == run.font() {
            if let Some(bold_italic) = bold_italic {
                if range == run.range {
                    *run = Run::new(range).with_font(bold_italic.clone());
                } else {
                    runs.push(Run::new(range).with_font(bold_italic.clone()));
                }
            }
        }
    } else if let Some(new_font) = new_font {
        runs.push(Run::new(range).with_font(new_font.clone()));
    }
}

fn apply_tag(
    runs: &mut RunSet,
    label: &str,
    argument: Option<&str>,
    range: Range<u32>,
    font: &FontResource,
) {
    match (label, argument) {
        ("i", None) => {
            if font.is_ok() {
                let font = font.data_ref();
                update_font(
                    runs,
                    range,
                    font.italic.as_ref(),
                    font.bold.as_ref(),
                    font.bold_italic.as_ref(),
                );
            }
        }
        ("b", None) => {
            if font.is_ok() {
                let font = font.data_ref();
                update_font(
                    runs,
                    range,
                    font.bold.as_ref(),
                    font.italic.as_ref(),
                    font.bold_italic.as_ref(),
                );
            }
        }
        ("size" | "s", Some(size)) => {
            if let Ok(size) = size.parse() {
                runs.push(Run::new(range).with_size(size));
            }
        }
        ("color" | "c", Some(color)) => {
            if let Ok(color) = color.parse() {
                runs.push(Run::new(range).with_brush(Brush::Solid(color)));
            }
        }
        ("shadow" | "sh", color) => {
            let mut run = Run::new(range).with_shadow(true);
            if let Some(color) = color.and_then(|c| c.parse().ok()) {
                run = run.with_shadow_brush(Brush::Solid(color));
            }
            runs.push(run);
        }
        _ => (),
    }
}

impl BBCode {
    pub fn build_formatted_text(self, font: FontResource) -> FormattedTextBuilder {
        let runs = self.build_runs(&font);
        FormattedTextBuilder::new(font)
            .with_text(self.text)
            .with_runs(runs)
    }
    pub fn build_runs(&self, font: &FontResource) -> RunSet {
        let mut runs = RunSet::default();
        let mut iter = self.tags.iter();
        while let Some(tag) = iter.next() {
            if tag.is_close {
                continue;
            }
            if let Some(close) = find_close(&tag.label, iter.clone()) {
                let start_pos = self.text[0..tag.position].chars().count() as u32;
                let end_pos = self.text[0..close.position].chars().count() as u32;
                apply_tag(
                    &mut runs,
                    &tag.label,
                    tag.argument.as_deref(),
                    start_pos..end_pos,
                    font,
                );
            }
        }
        runs
    }
}

#[cfg(test)]
mod test {
    use fyrox_core::color::Color;
    use fyrox_resource::untyped::ResourceKind;
    use uuid::Uuid;

    use crate::font::{Font, BUILT_IN_FONT};

    use super::*;
    #[test]
    fn test_built_in_font() {
        let font = BUILT_IN_FONT.resource();
        assert!(font.data_ref().bold.is_some());
        assert!(font.data_ref().italic.is_some());
        assert!(font.data_ref().bold_italic.is_some());
    }
    #[test]
    fn test_example() {
        let code: BBCode = "Here is [b]bold text[/b].".parse().unwrap();
        assert_eq!(&code.text, "Here is bold text.");
        assert_eq!(
            *code.tags,
            *&[
                BBTag {
                    position: 8,
                    data: BBTagData::open("b".into(), None)
                },
                BBTag {
                    position: 17,
                    data: BBTagData::close("b".into(), None)
                }
            ]
        );
    }
    #[test]
    fn test_example2() {
        let code: BBCode = "Here is [size = 24]big text[/ size= x ].".parse().unwrap();
        assert_eq!(&code.text, "Here is big text.");
        assert_eq!(
            *code.tags,
            *&[
                BBTag {
                    position: 8,
                    data: BBTagData::open("size".into(), Some("24".into()))
                },
                BBTag {
                    position: 16,
                    data: BBTagData::close("size".into(), Some("x".into()))
                }
            ]
        );
    }
    #[test]
    fn test_formatted() {
        let bold = FontResource::new_ok(Uuid::new_v4(), ResourceKind::Embedded, Font::default());
        let italic = FontResource::new_ok(Uuid::new_v4(), ResourceKind::Embedded, Font::default());
        let bold_italic =
            FontResource::new_ok(Uuid::new_v4(), ResourceKind::Embedded, Font::default());
        let font = FontResource::new_ok(
            Uuid::new_v4(),
            ResourceKind::Embedded,
            Font {
                bold: Some(bold.clone()),
                italic: Some(italic.clone()),
                bold_italic: Some(bold_italic.clone()),
                ..Font::default()
            },
        );
        let code: BBCode = "Here is [size=24]big text[/size].".parse().unwrap();
        let text = code.build_formatted_text(font.clone()).build();
        assert_eq!(**text.runs(), *&[Run::new(8..16).with_size(24.0)]);
        let code: BBCode = "Here is [shadow]big text[/shadow].".parse().unwrap();
        let text = code.build_formatted_text(font.clone()).build();
        assert_eq!(**text.runs(), *&[Run::new(8..16).with_shadow(true)]);
        let code: BBCode = "Here is [sh][s=24]big text[/s][/sh].".parse().unwrap();
        let text = code.build_formatted_text(font.clone()).build();
        assert_eq!(
            **text.runs(),
            *&[Run::new(8..16).with_shadow(true).with_size(24.0)]
        );
        let code: BBCode = "Here is [color=green]big text[/color].".parse().unwrap();
        let text = code.build_formatted_text(font.clone()).build();
        assert_eq!(
            **text.runs(),
            *&[Run::new(8..16).with_brush(Brush::Solid(Color::GREEN))]
        );
        let code: BBCode = "Here is [c=#010203]big text[/c].".parse().unwrap();
        let text = code.build_formatted_text(font.clone()).build();
        assert_eq!(
            **text.runs(),
            *&[Run::new(8..16).with_brush(Brush::Solid(Color::opaque(1, 2, 3)))]
        );
        let code: BBCode = "Here is [i]big text[/i].".parse().unwrap();
        let text = code.build_formatted_text(font.clone()).build();
        assert_eq!(**text.runs(), *&[Run::new(8..16).with_font(italic.clone())]);
        let code: BBCode = "Here is [b]big text[/b].".parse().unwrap();
        let text = code.build_formatted_text(font.clone()).build();
        assert_eq!(**text.runs(), *&[Run::new(8..16).with_font(bold.clone())]);
        let code: BBCode = "Here is [b][i]big text[/i][/b].".parse().unwrap();
        let text = code.build_formatted_text(font.clone()).build();
        assert_eq!(
            **text.runs(),
            *&[Run::new(8..16).with_font(bold_italic.clone())]
        );
        let code: BBCode = "Here is [i]big [b]text[/b]!!![/i]".parse().unwrap();
        let text = code.build_formatted_text(font.clone()).build();
        assert_eq!(
            **text.runs(),
            *&[
                Run::new(8..19).with_font(italic.clone()),
                Run::new(12..16).with_font(bold_italic.clone())
            ]
        );
    }
    #[test]
    fn test_nesting() {
        let font = FontResource::new_ok(Uuid::new_v4(), ResourceKind::Embedded, Font::default());
        let code: BBCode = "Here is [s=24]big [s=3]small[/s] text[/s]."
            .parse()
            .unwrap();
        let text = code.build_formatted_text(font.clone()).build();
        assert_eq!(
            **text.runs(),
            *&[
                Run::new(8..22).with_size(24.0),
                Run::new(12..17).with_size(3.0)
            ]
        );
    }
}
