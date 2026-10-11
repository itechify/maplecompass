//! The EXP bar (ADR 0005): the figures on the in-game EXP bar, "EXP. 160 [3.81%]",
//! read from the EXP Bar Region the player marks.
//!
//! 1. Find the brackets: the only yellow-green glyphs in the figures, a pair of narrow
//!    upright components with the same top and bottom. The bar's fill below is the same
//!    colour but never pairs up like that. If the mark takes in another pair (the HP
//!    figures to the left), the rightmost pair is the EXP's.
//! 2. The digits are neutral light grey on the dark slate bar, as tall as the brackets.
//!    The EXP is the cluster of ink just left of "[", stopping at the "EXP." label (taller
//!    than the brackets) or a gap wider than between digits; the percentage is the ink
//!    between the brackets.
//! 3. ocrs reads both as black-on-white lines, enlarged twice over. Only their digits
//!    count: the percentage always has two decimals, so its digits are hundredths of a
//!    percent.

use crate::Frame;
use crate::prep::{PAD, Rect};
use crate::recogniser::{Recogniser, frame_to_rgb};
use image::{GrayImage, Luma, RgbImage};
use serde::{Deserialize, Serialize};

/// What the EXP bar shows: the EXP gained in the current level, and how much of the
/// level that is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpBar {
    pub exp: u64,
    /// The percentage in hundredths: 381 for "3.81%".
    pub percent_hundredths: u32,
}

/// What one EXP Bar Region frame reads as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpBarReading {
    /// `None` when the figures can't be read whole (covered, or not in the region).
    pub bar: Option<ExpBar>,
    /// When the frame was captured, as in [`Frame::at_ms`].
    pub at_ms: u64,
}

/// Reads EXP Bar Region frames with the bundled recogniser (ADR 0002), which it shares
/// with the [`crate::TitleReader`]s.
pub struct ExpBarReader {
    recogniser: Recogniser,
}

impl ExpBarReader {
    pub fn bundled() -> Self {
        Self { recogniser: Recogniser::bundled() }
    }

    pub fn read(&self, frame: &Frame) -> ExpBarReading {
        let bar = frame_to_rgb(frame).and_then(|img| self.read_rgb(&img));
        ExpBarReading { bar, at_ms: frame.at_ms }
    }

    fn read_rgb(&self, img: &RgbImage) -> Option<ExpBar> {
        let prepared = prepare(img)?;
        let lines = self.recogniser.read_lines(&prepared.canvas, &[prepared.exp, prepared.percent])?;
        let text = |i: usize| lines.get(i)?.as_ref().map(|l| l.to_string());
        let exp = digits(&text(0)?)?;
        let percent_hundredths = percent(&text(1)?)?;
        let bar = ExpBar { exp: exp.parse().ok()?, percent_hundredths };
        consistent(bar).then_some(bar)
    }
}

/// A reading that could be on an EXP bar: under 100%, and no EXP at 0.00% unless the
/// level needs more than 10,000 EXP per hundredth (it never comes close).
fn consistent(bar: ExpBar) -> bool {
    bar.percent_hundredths < 10_000 && (bar.percent_hundredths > 0 || bar.exp < 10_000)
}

/// The digits of a figure as ocrs read it, with the letters it mistakes for digits in
/// this font put right. `None` if anything else is in it. Both figures are whole
/// numbers of digits, so a point or comma in one is only a separator: the EXP's
/// thousands comma ("8,435"), which ocrs reads as a point at this size, or the
/// percentage's decimal point.
fn digits(text: &str) -> Option<String> {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '0'..='9' => out.push(c),
            'O' | 'o' | 'D' => out.push('0'),
            'l' | 'I' | '|' | 'i' => out.push('1'),
            ' ' | ',' | '.' => {}
            _ => return None,
        }
    }
    (!out.is_empty() && out.len() <= 12).then_some(out)
}

/// Hundredths of a percent from "3.81%": the digits before the "%". The point may be
/// lost (it is one pixel square); the two decimals never are.
fn percent(text: &str) -> Option<u32> {
    let digits = digits(text.trim().trim_end_matches(['%', ')', ']']))?;
    (digits.len() >= 3).then(|| digits.parse().ok()).flatten()
}

/// The figures, masked and boxed for ocrs.
struct Prepared {
    canvas: GrayImage,
    exp: Rect,
    percent: Rect,
}

/// Bracket colour: yellow-green, (150,198,61) at its core.
fn bracket(p: [u8; 3]) -> bool {
    let [r, g, b] = p.map(i32::from);
    g >= 120 && g - b >= 60 && g - r >= 25
}

fn luma(p: [u8; 3]) -> f32 {
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

/// Digit ink, 0.0 (the dark slate bar, luma ~80) ..= 1.0 (the light grey digits, ~215).
/// Only near-neutral pixels count: the bar's edges are bluish, the brackets green.
fn ink(p: [u8; 3]) -> f32 {
    let [r, g, b] = p.map(i32::from);
    if (b - r).abs() > INK_MAX_TINT || (g - r).abs() > INK_MAX_TINT {
        return 0.0;
    }
    ((luma(p) - INK_LUMA_LO) / (INK_LUMA_HI - INK_LUMA_LO)).clamp(0.0, 1.0)
}

const INK_MAX_TINT: i32 = 24;
const INK_LUMA_LO: f32 = 100.0;
const INK_LUMA_HI: f32 = 190.0;
/// A column holds a glyph when its ink in the band sums past this.
const COL_MIN_INK: f32 = 0.5;
/// A bracket is at most this wide, and at least this tall and twice as tall as wide.
const BRACKET_MAX_W: u32 = 4;
const BRACKET_MIN_H: u32 = 6;
/// Per row of bracket height: glyphs of one figure are fewer ink-free columns apart than
/// this (digits touch or nearly; the "EXP." label ends 4 columns before them on the
/// samples)...
const GLYPH_GAP_PER_H: f32 = 0.2;
/// ...and a figure never has a gap wider than this.
const SPACE_PER_H: f32 = 0.6;
/// The "EXP." label rises this many rows above the brackets; digits never do.
const LABEL_RISE: u32 = 3;
/// The masked figures are enlarged this many times before ocrs reads them. At their
/// own 11-13 rows, ocrs read the 7 of "1.74%" as a 2 on the 2026-10-07 recording; every
/// sample reads right enlarged anywhere from 1.25x to 2.25x.
const UPSCALE: u32 = 2;

/// A connected run of bracket-coloured pixels: its bounding box.
#[derive(Debug, Clone, Copy)]
struct Blob {
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
}

/// Bounding boxes of the bracket-coloured components (4-connected).
fn blobs(img: &RgbImage) -> Vec<Blob> {
    let (w, h) = img.dimensions();
    let mut seen = vec![false; (w * h) as usize];
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if seen[(y * w + x) as usize] || !bracket(img.get_pixel(x, y).0) {
                continue;
            }
            let mut blob = Blob { x0: x, x1: x, y0: y, y1: y };
            let mut stack = vec![(x, y)];
            seen[(y * w + x) as usize] = true;
            while let Some((cx, cy)) = stack.pop() {
                blob = Blob { x0: blob.x0.min(cx), x1: blob.x1.max(cx), y0: blob.y0.min(cy), y1: blob.y1.max(cy) };
                let next = [(cx.wrapping_sub(1), cy), (cx + 1, cy), (cx, cy.wrapping_sub(1)), (cx, cy + 1)];
                for (nx, ny) in next {
                    if nx < w && ny < h && !seen[(ny * w + nx) as usize] && bracket(img.get_pixel(nx, ny).0) {
                        seen[(ny * w + nx) as usize] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            out.push(blob);
        }
    }
    out
}

/// The "[" and "]" of the figures: the rightmost pair of bracket-shaped components with
/// the same top and bottom and room for a digit between them (a stray component on
/// other rows between them doesn't break the pair).
fn brackets(img: &RgbImage) -> Option<(Blob, Blob)> {
    let mut shaped: Vec<Blob> = blobs(img)
        .into_iter()
        .filter(|b| {
            let (w, h) = (b.x1 - b.x0 + 1, b.y1 - b.y0 + 1);
            w <= BRACKET_MAX_W && h >= BRACKET_MIN_H && h >= 2 * w
        })
        .collect();
    shaped.sort_by_key(|b| b.x0);
    let pairs = |open: &Blob, close: &Blob| {
        open.y0.abs_diff(close.y0) <= 1 && open.y1.abs_diff(close.y1) <= 1 && close.x0 > open.x1 + 3
    };
    shaped.iter().rev().find_map(|close| {
        let open = shaped.iter().rev().find(|open| pairs(open, close))?;
        Some((*open, *close))
    })
}

fn prepare(img: &RgbImage) -> Option<Prepared> {
    let (open, close) = brackets(img)?;
    // The digits' rows: the brackets', with a row either side for anti-aliasing.
    let y0 = open.y0.min(close.y0).saturating_sub(1);
    let y1 = (open.y1.max(close.y1) + 2).min(img.height());
    let col_ink = |x: u32| (y0..y1).map(|y| ink(img.get_pixel(x, y).0)).sum::<f32>();
    let inked = |x: u32| col_ink(x) > COL_MIN_INK;
    let rises = |x: u32| {
        (y0.saturating_sub(LABEL_RISE)..y0).any(|y| ink(img.get_pixel(x, y).0) >= 0.5)
    };

    // The percentage: the ink between the brackets.
    let inside: Vec<u32> = (open.x1 + 1..close.x0).filter(|&x| inked(x)).collect();
    let (px0, px1) = (*inside.first()?, *inside.last()? + 1);

    // The EXP: the clusters of ink left of "[", back to the label (any cluster of it
    // rises above the brackets) or a gap wider than a space.
    let h = (open.y1 - open.y0 + 1) as f32;
    let split = (h * GLYPH_GAP_PER_H).ceil() as usize;
    let max_gap = (h * SPACE_PER_H).ceil() as u32;
    let on: Vec<bool> = (0..open.x0).map(inked).collect();
    let mut number: Option<(u32, u32)> = None;
    for (x0, x1) in clusters(&on, split).into_iter().rev() {
        let far = number.is_some_and(|(n0, _)| n0 - x1 > max_gap);
        if far || (x0..x1).any(rises) {
            break;
        }
        number = Some((x0, number.map_or(x1, |(_, n1)| n1)));
    }
    let (ex0, ex1) = number?;

    // Black on white, from the EXP's first column to the percentage's last.
    let (cx0, cw, ch) = (ex0, px1 - ex0, y1 - y0);
    let mut canvas = GrayImage::from_pixel(cw + 2 * PAD, ch + 2 * PAD, Luma([255]));
    for y in 0..ch {
        for x in 0..cw {
            let skip = (open.x0..=open.x1).contains(&(cx0 + x));
            let v = if skip { 0.0 } else { ink(img.get_pixel(cx0 + x, y0 + y).0) };
            canvas.put_pixel(x + PAD, y + PAD, Luma([(255.0 * (1.0 - v)).round() as u8]));
        }
    }
    let (w, h) = (canvas.width() * UPSCALE, canvas.height() * UPSCALE);
    let canvas = image::imageops::resize(&canvas, w, h, image::imageops::FilterType::Triangle);
    let line = |x0: u32, x1: u32| Rect {
        x: (x0 - cx0 + PAD) * UPSCALE,
        y: PAD * UPSCALE,
        w: (x1 - x0) * UPSCALE,
        h: ch * UPSCALE,
    };
    Some(Prepared { canvas, exp: line(ex0, ex1), percent: line(px0, px1) })
}

/// Runs of `on` columns as (start, end_exclusive), joining runs fewer than `split`
/// columns apart.
fn clusters(on: &[bool], split: usize) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for (x, _) in on.iter().enumerate().filter(|(_, on)| **on) {
        let x = x as u32;
        match out.last_mut() {
            Some((_, end)) if (x - *end) < split as u32 => *end = x + 1,
            _ => out.push((x, x + 1)),
        }
    }
    out
}
