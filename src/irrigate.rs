use image::{Rgba, DynamicImage, ImageBuffer};
use crate::{PanelResult, PanelError};
use crate::crop::judge;
use std::collections::{VecDeque, HashSet, HashMap};
use std::ops::{Range, Add};

//pub type Flag = Option<u32>;
#[derive(PartialOrd, PartialEq, Copy, Clone, Ord, Eq, Hash, Debug)]
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
    /// panels
    territories: Option<HashMap<Flag, Area>>,
}

#[derive(Clone, Debug)]
pub struct Area {
    pub x_range: Range<u32>,
    pub y_range: Range<u32>,
}

impl Area {
    pub fn calibrate(&mut self, x: u32, y: u32) {
        if !self.x_range.contains(&x) {
            if x < self.x_range.start {
                self.x_range.start = x;
            } else if self.x_range.end <= x {
                self.x_range.end = x + 1;
            } else {
                unreachable!()
            }
        }
        if !self.y_range.contains(&y) {
            if y < self.y_range.start {
                self.y_range.start = y;
            } else if self.y_range.end <= y {
                self.y_range.end = y + 1;
            } else {
                unreachable!()
            }
        }
    }
}

impl Add for Area {
    type Output = Area;

    fn add(self, rhs: Self) -> Self::Output {
        use std::cmp::{min, max};

        let x_min = min(self.x_range.start, rhs.x_range.start);
        let x_max = max(self.x_range.end, rhs.x_range.end);
        let x_range = x_min..x_max;

        let y_min = min(self.y_range.start, rhs.y_range.start);
        let y_max = max(self.y_range.end, rhs.y_range.end);
        let y_range = y_min..y_max;

        Self { x_range, y_range }
    }
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

        let territories = None;

        Ok(Self { img, x_range: ranges.0..ranges.1, y_range: ranges.2..ranges.3, queue: VecDeque::new(), zero_point, color_tolerance, reference_value, image_table, territories })
    }

    pub fn run(&mut self) -> Vec<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.flood_fill();
        self.labelling();
        self.get_panels()
    }

    /// フレームの読み取り
    fn flood_fill(&mut self) {
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
    fn labelling(&mut self) {
        let mut lookup_table = [None; 200];
//        let mut area_table = [(0..0,0..0);200];
        let mut territories: HashMap<Flag, Area> = HashMap::with_capacity(200);

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
                    // エリア情報
                    if let Some(territory) = territories.get_mut(&label) {
                        territory.calibrate(x, y);
                    } else {
                        territories.insert(label, Area {
                            x_range: x..x + 1,
                            y_range: y..y + 1,
                        });
                    }
                }
            }
        }

        for i in (0..lookup_table.len()).rev() {
            if let Some(label) = lookup_table[i] {
                // エリア情報をmarge
                {
                    let origin_key = Flag::Territory(label);
                    let sub_key = Flag::Territory(i as u32);

                    if let Some(area_origin) = territories.get(&origin_key) {
                        if let Some(area_sub) = territories.get(&sub_key) {
                            let area = area_origin.clone() + area_sub.clone();
                            territories.insert(origin_key, area);
                            territories.remove(&sub_key);
                        }
                    }
                }

                for y in self.y_range.clone() {
                    for x in self.x_range.clone() {
                        if self.image_table[y as usize][x as usize] == Some(Flag::Territory(i as u32)) {
                            self.image_table[y as usize][x as usize] = Some(Flag::Territory(label));
                        }
                    }
                }
            }
        }

        self.territories = Some(territories);
    }

    /// コマの取得
    fn get_panels(&mut self) -> Vec<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let mut panels = Vec::new();

        let territories = self.territories.as_ref().unwrap();

        for (flag, area) in territories {
            let width = area.x_range.end - area.x_range.start;
            let height = area.y_range.end - area.y_range.start;

            let mut image_buf: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);

            // 移植
            for y_from in area.y_range.clone() {
                for x_from in area.x_range.clone() {
                    if self.image_table[y_from as usize][x_from as usize] == Some(*flag) {
                        let pixel = self.get_pixel(x_from, y_from).unwrap();

                        let x_to = x_from - area.x_range.start;
                        let y_to = y_from - area.y_range.start;

                        image_buf.put_pixel(x_to, y_to, pixel);
                    }
                }
            }

            panels.push(image_buf);
        }

        panels
    }

    /// ラベリング
    #[deprecated]
    fn labelling_alt(&mut self) {
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

    #[deprecated]
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

#[cfg(test)]
mod tests {
    use image::Rgba;

    use test::Bencher;
    use crate::{crop::crop, irrigate::Labeler};

    #[bench]
    fn crop_bench(b: &mut Bencher) {
        b.iter(|| {
            let mut img = image::open("panels.png").unwrap();
            let _ranges = crop(&mut img, 100, &Rgba([255, 255, 255, 255])).unwrap();
        })
    }

    #[bench]
    fn irrigate_bench(b: &mut Bencher) {
        b.iter(|| {
            let mut img = image::open("panels.png").unwrap();
            let ranges = crop(&mut img, 100, &Rgba([255, 255, 255, 255])).unwrap();
            Labeler::new(&img, ranges, 100, (200, 615)).unwrap().flood_fill();
        });
    }

    #[bench]
    fn labelling_bench(b: &mut Bencher) {
        b.iter(|| {
            let mut img = image::open("panels.png").unwrap();
            let ranges = crop(&mut img, 100, &Rgba([255, 255, 255, 255])).unwrap();
            let mut labeler = Labeler::new(&img, ranges, 100, (200, 615)).unwrap();
            labeler.flood_fill();
            labeler.labelling();
        });
    }

    #[bench]
    fn panelling_bench(b: &mut Bencher) {
        b.iter(|| {
            let mut img = image::open("panels.png").unwrap();
            let ranges = crop(&mut img, 100, &Rgba([255, 255, 255, 255])).unwrap();
            let mut labeler = Labeler::new(&img, ranges, 100, (200, 615)).unwrap();
            labeler.run();
        })
    }
}
