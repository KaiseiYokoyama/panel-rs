use image::{Rgba, DynamicImage};
use crate::{PanelResult, PanelError};

/// 余白の削除
/// * image - 対象のイメージ
/// * color-tolerance - 許容する色差のノルム
/// * zero_point - 基準色の存在する座標
pub fn crop(image: &mut DynamicImage, color_tolerance: u32, zero_point: (u32, u32)) -> PanelResult<DynamicImage> {
    let mut img = image.to_rgba();

    // 基準色
    let standard = if zero_point.0 > img.width() || zero_point.1 > img.height() {
        return Err(PanelError::RangeError(format!("image: [{},{}], zero_point: ({},{})", img.width(), img.height(), zero_point.0, zero_point.1)));
    } else { img.get_pixel(zero_point.0, zero_point.1).clone() };

    let (mut left, mut right, mut top, mut bottom) = (None, None, None, None);
    for (row, mut pixels) in img.enumerate_rows_mut() {}


    for (row, mut pixels) in img.enumerate_rows_mut() {
        pixels.for_each(|(x, y, pixel)| {
            if !judge(&pixel, &standard, color_tolerance) {
                if let Some(val) = left {
                    if val < x {
                        left = Some(x);
                    }
                }
                if let Some(val) = right {
                    if val > x {
                        right = Some(x);
                    }
                }
                if let Some(val) = top {
                    if val < y {
                        top = Some(y);
                    }
                }
                if let Some(val) = bottom {
                    if val > y {
                        bottom = Some(y);
                    }
                }
            }
        });
    }

    Ok(image.crop(left.unwrap_or(0),
                  top.unwrap_or(0),
                  right.unwrap_or(img.width()) - (left.unwrap_or(0)),
                  bottom.unwrap_or(img.height()) - (top.unwrap_or(0))))
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