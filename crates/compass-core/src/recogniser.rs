//! Title recogniser (ADR 0002): ocrs, recognition model only. We supply the two line
//! boxes from our own mask and line split, so ocrs' detection model is not shipped.

use crate::Frame;
use crate::prep::{Numeral, Prepared, prepare};
use image::RgbImage;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams, TextItem, TextLine};
use rten::Model;
use rten_imageproc::{RectF, RotatedRect};

/// The bundled ocrs recognition model (CC-BY-SA-4.0, see `models/NOTICE.md`).
static RECOGNITION_MODEL: &[u8] = include_bytes!("../models/text-recognition.onnx");

/// Margin around each line box handed to ocrs (it fills outside the box with black).
const LINE_MARGIN: f32 = 3.0;

/// The two raw text lines read from the minimap title, before they are snapped to a
/// real map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleLines {
    pub street: String,
    pub map: String,
}

/// What one Title Region frame reads as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleReading {
    /// `None` when the frame is unreadable (loading screen, minimap hidden or covered).
    pub lines: Option<TitleLines>,
    /// When the frame was captured, as in [`Frame::at_ms`].
    pub at_ms: u64,
}

/// Reads Title Region frames with the bundled recogniser (ADR 0002). Reading takes
/// tens of milliseconds, so the shell keeps one on its own thread and hands the core
/// each [`TitleReading`] (`Compass::on_reading`).
pub struct TitleReader {
    recogniser: Recogniser,
}

impl TitleReader {
    pub fn bundled() -> Self {
        Self { recogniser: Recogniser::bundled() }
    }

    pub fn read(&self, frame: &Frame) -> TitleReading {
        let lines = frame_to_rgb(frame).and_then(|img| self.recogniser.read(&img));
        TitleReading { lines, at_ms: frame.at_ms }
    }
}

fn frame_to_rgb(frame: &Frame) -> Option<RgbImage> {
    let (w, h) = (frame.width as usize, frame.height as usize);
    if frame.rgba.len() != w * h * 4 {
        return None;
    }
    let rgb = frame.rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    RgbImage::from_raw(frame.width, frame.height, rgb)
}

struct Recogniser {
    engine: OcrEngine,
}

impl Recogniser {
    pub fn bundled() -> Self {
        let model = Model::load(RECOGNITION_MODEL.to_vec()).expect("bundled recognition model loads");
        let engine = OcrEngine::new(OcrEngineParams {
            recognition_model: Some(model),
            ..Default::default()
        })
        .expect("ocrs engine builds");
        Self { engine }
    }

    /// Read the street line and the map line. `None` means unreadable.
    fn read(&self, img: &RgbImage) -> Option<TitleLines> {
        let prepared = prepare(img)?;
        self.read_prepared(&prepared)
    }

    fn read_prepared(&self, p: &Prepared) -> Option<TitleLines> {
        let (w, h) = p.canvas.dimensions();
        let src = ImageSource::from_bytes(p.canvas.as_raw(), (w, h)).ok()?;
        let input = self.engine.prepare_input(src).ok()?;
        let boxes: Vec<Vec<RotatedRect>> = p
            .lines
            .iter()
            .map(|r| {
                vec![RotatedRect::from_rect(RectF::from_tlhw(
                    r.y as f32 - LINE_MARGIN,
                    r.x as f32 - LINE_MARGIN,
                    r.h as f32 + 2.0 * LINE_MARGIN,
                    r.w as f32 + 2.0 * LINE_MARGIN,
                ))]
            })
            .collect();
        let out = self.engine.recognize_text(&input, &boxes).ok()?;
        let line = |i: usize| out.get(i).and_then(|l| l.as_ref());
        let street = line(0).map(|l| l.to_string().trim().to_string());
        let map = match (line(1), p.numeral) {
            (line, Some(numeral)) => Some(with_numeral(line, numeral)),
            (Some(line), None) => Some(line.to_string().trim().to_string()),
            (None, None) => None,
        };
        let nonempty = |s: Option<String>| s.filter(|s| !s.is_empty());
        Some(TitleLines { street: nonempty(street)?, map: nonempty(map)? })
    }
}

/// The map line as read, with whatever ocrs made of its last word (an unknown glyph
/// for "II", say) replaced by the numeral counted from the mask.
fn with_numeral(line: Option<&TextLine>, numeral: Numeral) -> String {
    let before: String = line
        .map(|l| l.chars())
        .unwrap_or_default()
        .iter()
        .filter(|c| (c.rect.left() + c.rect.right()) < 2 * numeral.x as i32)
        .map(|c| c.char)
        .collect();
    format!("{} {}", before.trim(), "I".repeat(numeral.strokes)).trim().to_string()
}
