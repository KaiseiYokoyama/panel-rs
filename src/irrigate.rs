use image::{Rgba,  DynamicImage, ImageBuffer};
use crate::{PanelResult, PanelError};
use crate::crop::judge;
use std::collections::VecDeque;

//pub type Flag = Option<u32>;
#[derive(PartialOrd, PartialEq)]
pub enum Flag {
    Flame,
    Territory(u32),
}

pub type ImageTable = Vec<Vec<Option<Flag>>>;

pub struct Irrigater {
    /// 処理対象のイメージ
    img: ImageBuffer<Rgba<u8>, Vec<u8>>,
    /// 未処理のPixelの位置のキュー
    queue: VecDeque<(u32, u32)>,
    /// 0ポイント
    zero_point: (u32, u32),
    /// 色差の許容幅
    color_tolerance: u32,
    /// 基準色
    reference_value: Rgba<u8>,
    /// table
    image_table: ImageTable,
}

impl Irrigater {
    pub fn new(img: &DynamicImage, color_tolerance: u32, zero_point: (u32, u32)) -> PanelResult<Self> {
        let img = img.to_rgba();

        // 基準色
        let reference_value = if zero_point.0 > img.width() || zero_point.1 > img.height() {
            return Err(PanelError::RangeError(format!("image: [{},{}], zero_point: ({},{})", img.width(), img.height(), zero_point.0, zero_point.1)));
        } else { img.get_pixel(zero_point.0, zero_point.1).clone() };

        // table
        let mut image_table: ImageTable = Vec::with_capacity(img.height() as usize);
        for _i in 0..img.height() {
            let mut row = Vec::with_capacity(img.width() as usize);
            for _i in 0..img.width() {
                row.push(None);
            }
            image_table.push(row);
        }

        Ok(Self { img, queue: VecDeque::new(), zero_point, color_tolerance, reference_value, image_table })
    }

    /// フレームの読み取り
    pub fn flood_fill(&mut self) {
        let x = self.zero_point.0;
        let y = self.zero_point.1;
        if let Ok(_) = self.get_pixel(x, y) {
            self.queue.push_back((x, y));
            loop {
                self.flood_fill_step();
                if !self.queue.is_empty() {
                    break;
                }
            }
//            while !self.queue.is_empty() {
//                self.flood_fill_step();
//            }
        }
    }

    fn flood_fill_step(&mut self) {
        if let Some((x, y)) = self.queue.pop_front() {
//            println!("flood_fill_step: ({},{})", x, y);
            if let Ok(pixel) = self.get_pixel(x, y) {
                if self.image_table[y as usize][x as usize].is_none() && judge(&pixel, &self.reference_value, self.color_tolerance) {
                    self.image_table[y as usize][x as usize] = Some(Flag::Flame);

                    // 周囲4pixelを判定待ちの列に加える
                    let item = (x, y + 1);
                    if !self.queue.contains(&item) {
                        self.queue.push_back(item);
                    }
                    if x > 0 {
                        let item = (x - 1, y);
                        if !self.queue.contains(&item) {
                            self.queue.push_back(item);
                        }
                    }
                    if y > 0 {
                        let item = (x, y - 1);
                        if !self.queue.contains(&item) {
                            self.queue.push_back(item);
                        }
                    }
                    let item = (x + 1, y);
                    if !self.queue.contains(&item) {
                        self.queue.push_back(item);
                    }
                }
            }
//            println!("queue: {:?}", &self.queue);
        }
    }

    /// ピクセルを拾う
    fn get_pixel(&self, x: u32, y: u32) -> PanelResult<Rgba<u8>> {
        // 範囲外判定
        let width_range = 0..self.img.width();
        let height_range = 0..self.img.height();

        if width_range.contains(&x) && height_range.contains(&y) {
            Ok(self.img.get_pixel(x, y).clone())
        } else {
            Err(PanelError::RangeError(format!("image: [{},{}], zero_point: ({},{})", self.img.width(), self.img.height(), x, y)))
        }
    }
}

pub fn irrigate(image: &mut DynamicImage, color_tolerance: u32, zero_point: (u32, u32)) -> PanelResult<()> {
    let mut irrigater = Irrigater::new(&image, color_tolerance, zero_point)?;
    irrigater.flood_fill();

    // todo remove
//    let mut img = image.to_rgba();
//    let red = Rgba::from_channels(255, 0, 0, 255);
//    for i in 0..img.height() {
//        for j in 0..img.width() {
//            if irrigater.image_table[i as usize][j as usize] == Some(Flag::Flame) {
//                img.put_pixel(j, i, red.clone());
//            }
//        }
//    }
//
//    img.save("panels-irrigated.png").unwrap();

    Ok(())
}