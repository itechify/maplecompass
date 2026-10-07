//! Title preprocessing (from the recogniser spike, issue #2).
//!
//! 1. Locate the title band: the rows at the top of the minimap where the flat
//!    light-blue panel colour (164,200,218) dominates. No band = minimap hidden or
//!    covered = unreadable.
//! 2. Trim horizontally where the panel ends (world graphics right of a narrow minimap
//!    are never read), and skip past the map icon.
//! 3. Colour mask: the title text is neutral light grey (~223,223,223, B-R ~ 0); the
//!    panel is B-R = 54 and the outline a darker blue (B-R 60..80). So "ink" = how
//!    neutral the pixel is, gated on luma. Rendered as black text on white.
//! 4. Split the two lines (street, map) with a horizontal projection profile.

use image::{GrayImage, Luma, RgbImage};

/// Panel background colour (as in the sample screenshots) and per-channel tolerance.
/// Live GDI captures show the same colour uniformly darker, (153,187,204) ≈ 0.935×, so
/// the panel test matches the colour at any brightness within `PANEL_SCALE`.
const PANEL_RGB: [f32; 3] = [164.0, 200.0, 218.0];
const PANEL_TOL: f32 = 10.0;
const PANEL_SCALE: std::ops::RangeInclusive<f32> = 0.8..=1.1;
/// Search only this top-left area of a frame for the minimap.
const SEARCH_W: u32 = 900;
const SEARCH_H: u32 = 400;
/// The widest panel-colour row of the band has at least this many pixels.
const BAND_MIN_PANEL_PX: u32 = 120;
/// Every row of the title band has at least this many panel-colour pixels (rows
/// through a long name keep only ~80).
const BAND_ROW_MIN_PX: u32 = 30;
/// Rows ignored at the top and bottom of the band before masking.
const BAND_EDGE: u32 = 2;
/// Minimum height of the title band (rejects the 1-px rule above the header).
const BAND_MIN_H: u32 = 25;
/// "Very dark" (icon frame) luma, and how many such pixels make a column part of the icon.
const ICON_DARK_LUMA: f32 = 70.0;
const ICON_MIN_DARK_PX: u32 = 4;
/// The icon is within this many px of the panel's left edge.
const ICON_MAX_RIGHT: u32 = 120;
/// A column right of the icon with less total ink than this is the icon-to-text gap...
const ICON_GAP_MAX_INK: f32 = 0.05;
/// ...and the gap is at least this many consecutive such columns.
const ICON_GAP_COLS: u32 = 3;
/// Ink mask thresholds.
const INK_BR_HI: f32 = 48.0;
const INK_BR_SPAN: f32 = 40.0;
const INK_MIN_LUMA: f32 = 170.0;
/// A row counts as text if its summed ink exceeds this.
const LINE_MIN_INK: f32 = 1.5;
/// A column of a line counts as text if its summed ink exceeds this.
const COL_MIN_INK: f32 = 0.3;
/// White padding around the masked text canvas.
pub const PAD: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// A title ready for recognition: black-on-white ink canvas plus the street line and
/// map line boxes in it (canvas coordinates, tight ink bounds).
pub struct Prepared {
    pub canvas: GrayImage,
    pub lines: [Rect; 2],
}

fn luma(p: [u8; 3]) -> f32 {
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

fn is_panel(p: [u8; 3]) -> bool {
    // Brightness relative to the reference panel colour, taken from the blue channel.
    let k = p[2] as f32 / PANEL_RGB[2];
    PANEL_SCALE.contains(&k) && (0..2).all(|i| (p[i] as f32 - k * PANEL_RGB[i]).abs() <= PANEL_TOL)
}

/// 0.0 (background) ..= 1.0 (title text).
fn ink(p: [u8; 3]) -> f32 {
    if luma(p) < INK_MIN_LUMA {
        return 0.0;
    }
    let br = p[2] as f32 - p[0] as f32;
    ((INK_BR_HI - br) / INK_BR_SPAN).clamp(0.0, 1.0)
}

/// Find the title band (panel rows, trimmed to the panel's x extent).
fn locate_title(img: &RgbImage) -> Option<Rect> {
    let sw = img.width().min(SEARCH_W);
    let sh = img.height().min(SEARCH_H);
    let counts: Vec<u32> = (0..sh)
        .map(|y| (0..sw).filter(|&x| is_panel(img.get_pixel(x, y).0)).count() as u32)
        .collect();
    let on: Vec<bool> = counts.iter().map(|&c| c >= BAND_ROW_MIN_PX).collect();
    let (y0, y1) = runs(&on).into_iter().find(|&(a, b)| {
        b - a >= BAND_MIN_H && counts[a as usize..b as usize].iter().any(|&c| c >= BAND_MIN_PANEL_PX)
    })?;
    let h = y1 - y0;
    // Panel x extent: columns where the panel colour fills at least a third of the band.
    let cols: Vec<u32> = (0..sw)
        .filter(|&x| (y0..y1).filter(|&y| is_panel(img.get_pixel(x, y).0)).count() as u32 * 3 >= h)
        .collect();
    let (x0, x1) = (*cols.first()?, *cols.last()? + 1);
    Some(Rect { x: x0, y: y0, w: x1 - x0, h })
}

/// Group consecutive indices where `on[i]` into (start, end_exclusive) runs.
fn runs(on: &[bool]) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < on.len() {
        if on[i] {
            let s = i;
            while i < on.len() && on[i] {
                i += 1;
            }
            out.push((s as u32, i as u32));
        } else {
            i += 1;
        }
    }
    out
}

/// Locate, trim, mask and split a frame. `None` means unreadable.
pub fn prepare(img: &RgbImage) -> Option<Prepared> {
    let region = locate_title(img)?;
    let Rect { x: rx, w: rw, .. } = region;
    // Drop the band's outermost rows: after resampling they blend with the light
    // border next to the panel and read as a full-width line of ink.
    let (ry, rh) = (region.y + BAND_EDGE, region.h.checked_sub(2 * BAND_EDGE)?);
    // Skip the map icon: rightmost column near the left with several very dark pixels.
    let icon_right = (rx..(rx + ICON_MAX_RIGHT).min(rx + rw))
        .filter(|&x| {
            (ry..ry + rh).filter(|&y| luma(img.get_pixel(x, y).0) < ICON_DARK_LUMA).count() as u32
                >= ICON_MIN_DARK_PX
        })
        .max()
        .unwrap_or(rx);
    // Step past the icon's light anti-aliased rim: text starts after the first run of
    // ink-free columns (the icon-to-text gap).
    let col_ink = |x: u32| (ry..ry + rh).map(|y| ink(img.get_pixel(x, y).0)).sum::<f32>();
    let tx1 = rx + rw;
    let tx0 = (icon_right + 1..tx1.saturating_sub(ICON_GAP_COLS))
        .find(|&x| (x..x + ICON_GAP_COLS).all(|c| col_ink(c) < ICON_GAP_MAX_INK))?;
    if tx0 >= tx1 {
        return None;
    }
    let (tw, th) = (tx1 - tx0, rh);
    let inks: Vec<f32> = (0..th)
        .flat_map(|y| (0..tw).map(move |x| (x, y)))
        .map(|(x, y)| ink(img.get_pixel(tx0 + x, ry + y).0))
        .collect();
    let at = |x: u32, y: u32| inks[(y * tw + x) as usize];

    // Lines: horizontal projection; keep the two tallest runs, top to bottom.
    let row_on: Vec<bool> =
        (0..th).map(|y| (0..tw).map(|x| at(x, y)).sum::<f32>() > LINE_MIN_INK).collect();
    let mut row_runs = runs(&row_on);
    if row_runs.len() < 2 {
        return None; // not a two-line title
    }
    row_runs.sort_by_key(|r| std::cmp::Reverse(r.1 - r.0));
    row_runs.truncate(2);
    row_runs.sort();
    let line = |(ly0, ly1): (u32, u32)| -> Option<Rect> {
        let col_on: Vec<bool> =
            (0..tw).map(|x| (ly0..ly1).map(|y| at(x, y)).sum::<f32>() > COL_MIN_INK).collect();
        let first = col_on.iter().position(|&b| b)? as u32;
        let last = col_on.iter().rposition(|&b| b)? as u32 + 1;
        Some(Rect { x: first + PAD, y: ly0 + PAD, w: last - first, h: ly1 - ly0 })
    };
    let lines = [line(row_runs[0])?, line(row_runs[1])?];

    let mut canvas = GrayImage::from_pixel(tw + 2 * PAD, th + 2 * PAD, Luma([255]));
    for y in 0..th {
        for x in 0..tw {
            canvas.put_pixel(x + PAD, y + PAD, Luma([(255.0 * (1.0 - at(x, y))).round() as u8]));
        }
    }
    Some(Prepared { canvas, lines })
}
