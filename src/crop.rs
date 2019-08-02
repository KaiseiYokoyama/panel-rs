use image::{Rgba, RgbaImage, DynamicImage, ImageBuffer, Pixel, Primitive};
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

    // 上下の切り取り
    // 上から
    let mut y_top = None;
    for (row, mut pixels) in img.enumerate_rows_mut() {
        if pixels.all(|(_, _, pixel)| { judge(&pixel, &standard, color_tolerance) }) {
            y_top = Some(row);
        } else { break; }
    }

    // 下から
    let mut y_bottom = None;
    for y in (0..img.height()).rev() {
        let mut row = true;
        for x in 0..img.width() {
            row = row && judge(img.get_pixel(x, y), &standard, color_tolerance);
        }
        if row { y_bottom = Some(y); } else { break; }
    }

    // 左右の切り取り
    // 左から
    let mut x_top = None;
    for x in 0..img.width() {
        let mut column = true;
        for y in 0..img.height() {
            column = column && judge(img.get_pixel(x, y), &standard, color_tolerance);
        }
        if column { x_top = Some(x); } else { break; }
    }

    // 右から
    let mut x_bottom = None;
    for x in (0..img.width()).rev() {
        let mut column = true;
        for y in 0..img.height() {
            column = column && judge(img.get_pixel(x, y), &standard, color_tolerance);
        }
        if column { x_bottom = Some(x); } else { break; }
    }

    Ok(image.crop(x_top.unwrap_or(0) + 1,
                  y_top.unwrap_or(0) + 1,
                  x_bottom.unwrap_or(img.width()) - (x_top.unwrap_or(0) + 1),
                  y_bottom.unwrap_or(img.height()) - (y_top.unwrap_or(0) + 1)))
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