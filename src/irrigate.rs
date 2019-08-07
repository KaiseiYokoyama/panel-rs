use image::{Rgba, DynamicImage, ImageBuffer};
use crate::{PanelResult, PanelError};
use crate::crop::judge;
use std::collections::{VecDeque, HashSet};
use std::ops::Range;

//pub type Flag = Option<u32>;
#[derive(PartialOrd, PartialEq, Copy, Clone, Ord, Eq)]
pub enum Flag {
    Flame,
    Territory(u32),
}

impl Flag {
    pub fn next(&self) -> Self {
        match &self {
            Flag::Flame => Flag::Territory(0),
            Flag::Territory(i) => Flag::Territory(i + 1),
        }
    }
}

pub type ImageTable = Vec<Vec<Option<Flag>>>;

pub struct Labeler {
    /// 処理対象のイメージ
    img: ImageBuffer<Rgba<u8>, Vec<u8>>,
    /// 処理範囲
    x_range: Range<u32>,
    /// 処理範囲
    y_range: Range<u32>,
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

impl Labeler {
    pub fn new(img: &DynamicImage, ranges: (u32, u32, u32, u32), color_tolerance: u32, zero_point: (u32, u32)) -> PanelResult<Self> {
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

        println!("x_range: {:?}", &ranges.0);
        println!("y_range: {:?}", &ranges.1);

        Ok(Self { img, x_range: ranges.0..ranges.1, y_range: ranges.2..ranges.3, queue: VecDeque::new(), zero_point, color_tolerance, reference_value, image_table })
    }

    pub fn run(&mut self) {
        self.flood_fill();
        self.labelling();
    }

    /// フレームの読み取り
    pub fn flood_fill(&mut self) {
        let x = self.zero_point.0;
        let y = self.zero_point.1;
        if let Ok(_) = self.get_pixel(x, y) {
            self.queue.push_back((x, y));
            loop {
                self.flood_fill_step();
                if self.queue.is_empty() {
                    break;
                }
            }
        }
    }

    fn flood_fill_step(&mut self) {
        if let Some((x, y)) = self.queue.pop_front() {
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
        }
    }

    /// ラベリング
    pub fn labelling(&mut self) {
        let mut lookup_table = [None; 200];

        let mut last = Flag::Flame;
        for y in self.y_range.clone() {
            for x in self.x_range.clone() {
                if self.image_table[y as usize][x as usize] != Some(Flag::Flame) {
                    let upper = if y >= 1 { self.get_label(x, y - 1) } else { None };
                    let before = if x >= 1 { self.get_label(x - 1, y) } else { None };
                    let label = match (upper, before) {
                        (None, None) | (Some(Flag::Flame), Some(Flag::Flame)) => {
                            last = last.next();
                            last.next()
                        }
                        (Some(flag1), Some(flag2)) =>
                            if flag1 == Flag::Flame {
                                flag2
                            } else if flag2 == Flag::Flame {
                                flag1
                            } else {
                                if flag1 == flag2 { flag1 } else {
                                    let max = std::cmp::max(flag1, flag2);
                                    let min = std::cmp::min(flag1, flag2);
                                    if let (Flag::Territory(max), Flag::Territory(min)) = (max, min) {
                                        if lookup_table[max as usize] != Some(min) {
                                            lookup_table[max as usize] = Some(min);
                                        }

                                        std::cmp::min(flag1, flag2)
                                    } else { unreachable!() }
                                }
                            }
                        (None, Some(flag)) | (Some(flag), None) =>
                            if flag == Flag::Flame {
                                last = last.next();
                                last.next()
                            } else { flag }
                    };

                    self.image_table[y as usize][x as usize] = Some(label);
                }
            }
        }

        for i in (0..lookup_table.len()).rev() {
            if let Some(label) = lookup_table[i] {
                for y in self.y_range.clone() {
                    for x in self.x_range.clone() {
                        if self.image_table[y as usize][x as usize] == Some(Flag::Territory(i as u32)) {
                            self.image_table[y as usize][x as usize] = Some(Flag::Territory(label));
                        }
                    }
                }
            }
        }
    }

    /// ラベリング
    pub fn labelling_alt(&mut self) {
        let mut flag = Flag::Flame;
        for y in self.y_range.clone() {
            for x in self.x_range.clone() {
                if None == self.get_label(x, y) {
                    flag = flag.next();

                    let mut queue = VecDeque::new();
                    queue.push_back((x, y));

                    loop {
                        self.panel_flood_fill(&mut queue, flag);
                        if queue.is_empty() {
                            break;
                        }
                    }
                }
            }
        }
    }

    fn panel_flood_fill(&mut self, queue: &mut VecDeque<(u32, u32)>, flag: Flag) {
        if let Some((x, y)) = queue.pop_front() {
            if self.x_range.contains(&x) && self.y_range.contains(&y) {
                if Some(Flag::Flame) != self.get_label(x, y) && self.image_table[y as usize][x as usize].is_none() {
                    self.image_table[y as usize][x as usize] = Some(flag);

                    // 周囲4pixelを判定待ちの列に加える
                    let item = (x, y + 1);
                    if !queue.contains(&item) {
                        queue.push_back(item);
                    }
                    if x > 0 {
                        let item = (x - 1, y);
                        if !queue.contains(&item) {
                            queue.push_back(item);
                        }
                    }
                    if y > 0 {
                        let item = (x, y - 1);
                        if !queue.contains(&item) {
                            queue.push_back(item);
                        }
                    }
                    let item = (x + 1, y);
                    if !queue.contains(&item) {
                        queue.push_back(item);
                    }
                }
            }
        }
    }

    /// ピクセルを拾う
    fn get_pixel(&self, x: u32, y: u32) -> PanelResult<Rgba<u8>> {
        // 範囲外判定
//        let width_range = 0..self.img.width();
//        let height_range = 0..self.img.height();

        if self.x_range.contains(&x) && self.y_range.contains(&y) {
            Ok(self.img.get_pixel(x, y).clone())
        } else {
            Err(PanelError::RangeError(format!("image: [{:?},{:?}], ({},{})", &self.x_range, &self.y_range, x, y)))
        }
    }

    /// ラベルを拾う
    fn get_label(&self, x: u32, y: u32) -> Option<Flag> {
        if self.x_range.contains(&x) && self.y_range.contains(&y) {
            self.image_table[y as usize][x as usize]
        } else {
            None
        }
    }

    /// mapを出力
    pub fn output_map(&self) {
        let mut img = self.img.clone();
        let red = Rgba([255, 0, 0, 255]);
        let colors = vec![
            // purple
            Rgba([156, 30, 176, 255]),
            // blue
            Rgba([33, 150, 243, 255]),
            // teal
            Rgba([0, 150, 136, 255]),
            // green
            Rgba([76, 175, 80, 255]),
            // lime
            Rgba([205, 220, 57, 255]),
            // amber
            Rgba([255, 193, 7, 255]),
        ];

        for i in 0..img.height() {
            for j in 0..img.width() {
                match self.image_table[i as usize][j as usize] {
                    Some(Flag::Flame) => {
                        img.put_pixel(j, i, red.clone());
                    }
                    Some(Flag::Territory(label)) => {
                        img.put_pixel(j, i, colors.get(label as usize % colors.len()).cloned().unwrap());
                    }
                    _ => {}
                }
            }
        }

        img.save("panels-irrigated.png").unwrap();
    }
}
