use image::{Rgba, DynamicImage};
use crate::{PanelResult, PanelError};
use std::ops::Range;

/// 余白の削除
/// * image - 対象のイメージ
/// * color-tolerance - 許容する色差のノルム
/// * zero_point - 基準色の存在する座標
pub fn crop(image: &DynamicImage, color_tolerance: u32, reference: &Rgba<u8>) -> PanelResult<(u32, u32, u32, u32)> {
    let mut img = image.to_rgba();

    let (mut left, mut right, mut top, mut bottom) = (None, 0, None, 0);

    for (_, pixels) in img.enumerate_rows() {
        pixels.for_each(|(x, y, pixel)| {
            if !judge(&pixel, reference, color_tolerance) {
                if let Some(val) = left {
                    if x < val {
                        left = Some(x);
                    }
                } else {
                    left = Some(x);
                }
                if x > right {
                    right = x + 1;
                }
                if None == top {
                    top = Some(y);
                }
                if y > bottom {
                    bottom = y + 1;
                }
            }
        });
    }

    Ok((left.unwrap_or(0), right,
        top.unwrap_or(0), bottom))
}

pub fn judge(pixel: &Rgba<u8>, standard: &Rgba<u8>, tolerance: u32) -> bool {
    let (dr, dg, db, da) = {
        let Rgba([r, g, b, a]) = pixel.clone();
        let Rgba([sr, sg, sb, sa]) = standard.clone();

        ((sr as i32 - r as i32), (sg as i32 - g as i32), (sb as i32 - b as i32), (sa as i32 - a as i32))
    };

    let dist2 = (dr * dr) + (dg * dg) + (db * db) + (da * da);

    dist2 < tolerance as i32 * tolerance as i32
}