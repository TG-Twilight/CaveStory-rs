use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::path::PathBuf;

use byteorder::{LE, ReadBytesExt};

use crate::common::{FILE_TYPES, Rect};
use crate::engine_constants::EngineConstants;
use crate::framework::context::Context;
use crate::framework::error::GameError::ResourceLoadError;
use crate::framework::error::GameResult;
use crate::framework::filesystem;
use crate::graphics::font::{EMPTY_SYMBOLS, Font, Symbols, TextBuilderFlag};
use crate::graphics::texture_set::TextureSet;

#[derive(Debug)]
pub struct BMChar {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub x_offset: i16,
    pub y_offset: i16,
    pub x_advance: i16,
    pub page: u8,
    pub channel: u8,
}

#[derive(Debug)]
pub struct BMFontMetadata {
    pub pages: u16,
    pub font_size: i16,
    pub line_height: u16,
    pub base: u16,
    pub chars: HashMap<char, BMChar>,
}

#[repr(u8)]
pub enum BMFontBlockType {
    Unknown = 0,
    Info = 1,
    Common = 2,
    Pages = 3,
    Chars = 4,
    KerningPairs = 5,
}

impl From<u8> for BMFontBlockType {
    fn from(value: u8) -> Self {
        match value {
            1 => BMFontBlockType::Info,
            2 => BMFontBlockType::Common,
            3 => BMFontBlockType::Pages,
            4 => BMFontBlockType::Chars,
            5 => BMFontBlockType::KerningPairs,
            _ => BMFontBlockType::Unknown,
        }
    }
}

const MAGIC: [u8; 4] = [b'B', b'M', b'F', 3];

impl BMFontMetadata {
    pub fn load_from<R: io::Read + io::Seek>(mut data: R) -> GameResult<Self> {
        let mut magic = [0u8; 4];
        let mut pages = 0u16;
        let mut chars = HashMap::new();
        let mut font_size = 0i16;
        let mut line_height = 0u16;
        let mut base = 0u16;

        data.read_exact(&mut magic)?;

        if magic != MAGIC {
            return Err(ResourceLoadError("Invalid magic".to_owned()));
        }

        while let Ok(block_type) = data.read_u8() {
            let length = data.read_u32::<LE>()?;
            match BMFontBlockType::from(block_type) {
                BMFontBlockType::Info => {
                    font_size = data.read_i16::<LE>()?;

                    data.seek(io::SeekFrom::Current(length as i64 - 2))?;
                }
                BMFontBlockType::Common => {
                    line_height = data.read_u16::<LE>()?;
                    base = data.read_u16::<LE>()?;
                    data.seek(io::SeekFrom::Current(4))?;
                    pages = data.read_u16::<LE>()?;

                    data.seek(io::SeekFrom::Current(length as i64 - 10))?;
                }
                BMFontBlockType::Chars => {
                    let count = length / 20;
                    chars.reserve(count as usize);

                    for _ in 0..count {
                        let id = data.read_u32::<LE>()?;
                        let x = data.read_u16::<LE>()?;
                        let y = data.read_u16::<LE>()?;
                        let width = data.read_u16::<LE>()?;
                        let height = data.read_u16::<LE>()?;
                        let x_offset = data.read_i16::<LE>()?;
                        let y_offset = data.read_i16::<LE>()?;
                        let x_advance = data.read_i16::<LE>()?;
                        let page = data.read_u8()?;
                        let channel = data.read_u8()?;

                        if let Some(chr) = std::char::from_u32(id) {
                            chars.insert(
                                chr,
                                BMChar { x, y, width, height, x_offset, y_offset, x_advance, page, channel },
                            );
                        }
                    }
                }
                BMFontBlockType::Unknown => {
                    return Err(ResourceLoadError("Unknown block type.".to_owned()));
                }
                _ => {
                    data.seek(io::SeekFrom::Current(length as i64))?;
                }
            }
        }

        Ok(Self { pages, font_size, line_height, base, chars })
    }
}

pub struct BMFont {
    font: BMFontMetadata,
    font_scale: f32,
    pages: Vec<String>,
}

impl Font for BMFont {
    fn line_height(&self) -> f32 {
        self.font.line_height as f32 * self.font_scale
    }

    fn compute_width(&self, text: &mut dyn Iterator<Item = char>, symbols: Option<&Symbols>) -> f32 {
        let mut offset_x = 0.0;

        if let Some(syms) = symbols {
            for chr in text {
                let rect_map_entry = syms.symbols.iter().find(|(c, _)| *c == chr);

                if let Some((_, rect)) = rect_map_entry {
                    offset_x += rect.width() as f32;
                } else if let Some(glyph) = self.font.chars.get(&chr) {
                    offset_x += glyph.x_advance as f32 * self.font_scale;
                }
            }
        } else {
            for chr in text {
                if let Some(glyph) = self.font.chars.get(&chr) {
                    offset_x += glyph.x_advance as f32 * self.font_scale;
                }
            }
        }

        offset_x
    }

    fn draw(
        &self,
        text: &mut dyn Iterator<Item = char>,
        mut x: f32,
        y: f32,
        scale: f32,
        box_width: f32,
        shadow_color: (u8, u8, u8, u8),
        color: (u8, u8, u8, u8),
        flags: TextBuilderFlag,
        constants: &EngineConstants,
        texture_set: &mut TextureSet,
        symbols: Option<Symbols>,
        ctx: &mut Context,
    ) -> GameResult {
        unsafe {
            static mut TEXT_BUF: Vec<char> = Vec::new();

            TEXT_BUF.clear();
            for c in text {
                TEXT_BUF.push(c);
            }

            if flags.centered() {
                let text_width = self.compute_width(&mut TEXT_BUF.iter().copied(), symbols.as_ref());

                x += (box_width - text_width) * 0.5;
            }

            if flags.shadow() {
                self.draw_text_line(
                    &mut TEXT_BUF.iter().copied(),
                    x + scale,
                    y + scale,
                    scale,
                    shadow_color,
                    constants,
                    texture_set,
                    symbols.as_ref(),
                    ctx,
                )?;
            }

            self.draw_text_line(
                &mut TEXT_BUF.iter().copied(),
                x,
                y,
                scale,
                color,
                constants,
                texture_set,
                symbols.as_ref(),
                ctx,
            )?;
        }

        Ok(())
    }
}

enum LineItem<'a> {
    Glyph(&'a BMChar),
    Symbol(&'a Rect<u16>),
}

impl BMFont {
    fn layout_line<'a>(&'a self, text: &mut dyn Iterator<Item = char>, symbols: &'a Symbols, scale: f32)
        -> Vec<(f32, LineItem<'a>)> {
        let mut result = Vec::new();
        let mut offset = 0.0;
        for chr in text {
            if let Some((_, rect)) = symbols.symbols.iter().find(|(c, _)| *c == chr) {
                result.push((offset, LineItem::Symbol(rect)));
                offset += rect.width() as f32 * scale;
            } else if let Some(glyph) = self.font.chars.get(&chr) {
                result.push((offset, LineItem::Glyph(glyph)));
                offset += glyph.x_advance as f32 * self.font_scale * scale;
            }
        }
        result
    }

    pub fn load(roots: &Vec<String>, desc_path: &str, ctx: &mut Context, font_scale: f32) -> GameResult<BMFont> {
        let full_path = PathBuf::from(desc_path);
        let desc_stem =
            full_path.file_stem().ok_or_else(|| ResourceLoadError("Cannot extract the file stem.".to_owned()))?;
        let stem = full_path.parent().unwrap_or(&full_path).join(desc_stem);

        let font = BMFontMetadata::load_from(filesystem::open_find(ctx, roots, &full_path)?)?;
        let mut pages = Vec::new();

        let (zeros, _, _) = FILE_TYPES
            .iter()
            .map(|ext| (1, ext, format!("{}_0{}", stem.to_string_lossy(), ext)))
            .find(|(_, _, path)| filesystem::exists_find(ctx, roots, &path))
            .or_else(|| {
                FILE_TYPES
                    .iter()
                    .map(|ext| (2, ext, format!("{}_00{}", stem.to_string_lossy(), ext)))
                    .find(|(_, _, path)| filesystem::exists_find(ctx, roots, &path))
            })
            .ok_or_else(|| ResourceLoadError(format!("Cannot find glyph atlas 0 for font: {:?}", desc_path)))?;

        for i in 0..font.pages {
            let page_path = format!("{}_{:02$}", stem.to_string_lossy(), i, zeros);

            pages.push(page_path);
        }

        Ok(Self { font, font_scale, pages })
    }

    fn draw_text_line(
        &self,
        iter: &mut dyn Iterator<Item = char>,
        x: f32,
        y: f32,
        scale: f32,
        color: (u8, u8, u8, u8),
        constants: &EngineConstants,
        texture_set: &mut TextureSet,
        symbols: Option<&Symbols>,
        ctx: &mut Context,
    ) -> GameResult {
        let syms = symbols.unwrap_or(&EMPTY_SYMBOLS);
        // Resolve once, independent of font atlas pages. Symbols need no font glyph.
        let layout = self.layout_line(iter, syms, scale);
        let pages: HashSet<u8> = layout.iter().filter_map(|(_, item)| match item {
            LineItem::Glyph(glyph) => Some(glyph.page),
            LineItem::Symbol(_) => None,
        }).collect();

        for page in pages {
            let Some(page_tex) = self.pages.get(page as usize) else { continue; };
            let batch = texture_set.get_or_load_batch(ctx, constants, page_tex)?;
            for (offset, item) in &layout {
                if let LineItem::Glyph(glyph) = item {
                    if glyph.page == page {
                        batch.add_rect_scaled_tinted(
                            x + offset + glyph.x_offset as f32 * self.font_scale,
                            y + glyph.y_offset as f32 * self.font_scale,
                            color, self.font_scale * scale, self.font_scale * scale,
                            &Rect::new_size(glyph.x, glyph.y, glyph.width, glyph.height),
                        );
                    }
                }
            }
            batch.draw(ctx)?;
        }

        if !syms.texture.is_empty() && layout.iter().any(|(_, item)| matches!(item, LineItem::Symbol(_))) {
            let batch = texture_set.get_or_load_batch(ctx, constants, syms.texture)?;
            for (offset, item) in &layout {
                if let LineItem::Symbol(rect) = item {
                    batch.add_rect_scaled_tinted(
                        x + offset, y + (self.line_height() - rect.height() as f32) * scale / 2.0,
                        color, scale, scale, rect,
                    );
                }
            }
            batch.draw(ctx)?;
        }

        Ok(())
    }

    pub fn scale(&mut self, scale: f32) {
        self.font_scale = scale;
    }

    pub fn get_scale(&self) -> f32 {
        self.font_scale
    }
}

#[cfg(test)]
mod symbol_layout_tests {
    use super::*;
    #[test]
    fn symbols_without_font_glyphs_preserve_scaled_positions_across_pages() {
        let glyph = |page| BMChar { x: 0, y: 0, width: 8, height: 10, x_offset: 0,
            y_offset: 0, x_advance: 8, page, channel: 0 };
        let font = BMFont { font: BMFontMetadata { pages: 2, font_size: 10, line_height: 10,
            base: 0, chars: [('A', glyph(0)), ('B', glyph(1))].into_iter().collect() },
            font_scale: 1.0, pages: vec!["a".into(), "b".into()] };
        let symbols = Symbols { symbols: &[('\u{e000}', Rect::new_size(0, 0, 16, 16))], texture: "buttons" };
        let layout = font.layout_line(&mut "A\u{e000}B".chars(), &symbols, 2.0);
        assert_eq!(layout.len(), 3);
        assert_eq!(layout.iter().map(|(x, _)| *x).collect::<Vec<_>>(), vec![0.0, 16.0, 48.0]);
        assert_eq!(layout.iter().filter(|(_, item)| matches!(item, LineItem::Symbol(_))).count(), 1);
        assert_eq!(font.compute_width(&mut "A\u{e000}B".chars(), Some(&symbols)), 32.0);
        let only_symbol = font.layout_line(&mut "\u{e000}".chars(), &symbols, 1.0);
        assert_eq!(only_symbol.len(), 1);
        assert!(matches!(only_symbol[0].1, LineItem::Symbol(_)));
        let plain = font.layout_line(&mut "AB".chars(), &EMPTY_SYMBOLS, 2.0);
        assert_eq!(plain.iter().map(|(x, _)| *x).collect::<Vec<_>>(), vec![0.0, 16.0]);
        let override_glyph = Symbols { symbols: &[('A', Rect::new_size(0, 0, 16, 16)),
            ('\u{e001}', Rect::new_size(0, 0, 24, 16))], texture: "buttons" };
        let mixed = font.layout_line(&mut "A\u{e001}B".chars(), &override_glyph, 1.0);
        assert_eq!(mixed.iter().map(|(x, _)| *x).collect::<Vec<_>>(), vec![0.0, 16.0, 40.0]);
        assert_eq!(mixed.iter().filter(|(_, item)| matches!(item, LineItem::Symbol(_))).count(), 2);

    }
}
