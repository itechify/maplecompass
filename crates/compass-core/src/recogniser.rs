//! Title recogniser (ADR 0002): ocrs, recognition model only. We supply the two line
//! boxes from our own mask and line split, so ocrs' detection model is not shipped.

use crate::prep::{Prepared, prepare};
use image::RgbImage;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use rten_imageproc::{RectF, RotatedRect};

/// The bundled ocrs recognition model (CC-BY-SA-4.0, see `models/NOTICE.md`).
static RECOGNITION_MODEL: &[u8] = include_bytes!("../models/text-recognition.onnx");

/// Margin around each line box handed to ocrs (it fills outside the box with black).
const LINE_MARGIN: f32 = 3.0;

/// The two raw text lines read from the minimap title.
pub struct RawTitle {
    pub street: String,
    pub map: String,
}

pub struct Recogniser {
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
    pub fn read(&self, img: &RgbImage) -> Option<RawTitle> {
        let prepared = prepare(img)?;
        self.read_prepared(&prepared)
    }

    fn read_prepared(&self, p: &Prepared) -> Option<RawTitle> {
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
        let text = |i: usize| {
            out.get(i)
                .and_then(|l| l.as_ref())
                .map(|l| l.to_string().trim().to_string())
                .filter(|s| !s.is_empty())
        };
        Some(RawTitle { street: text(0)?, map: text(1)? })
    }
}
