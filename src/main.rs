#![feature(test)]

extern crate test;

use image::Rgba;
use crate::{crop::crop, irrigate::Labeler};

mod crop;
mod irrigate;

#[derive(Debug)]
pub enum PanelError {
    IOError(std::io::Error),
    ImageError(image::ImageError),
    RangeError(String),
}

impl From<std::io::Error> for PanelError {
    fn from(error: std::io::Error) -> Self {
        PanelError::IOError(error)
    }
}

impl From<image::ImageError> for PanelError {
    fn from(error: image::ImageError) -> Self {
        PanelError::ImageError(error)
    }
}

pub type PanelResult<T> = Result<T, PanelError>;

fn main() -> image::ImageResult<()> {
    match run("panels.png", 100, (0, 0)) {
        Err(PanelError::RangeError(string)) => println!("{}", string),
        Err(PanelError::IOError(err)) => println!("{:?}", err),
        Err(PanelError::ImageError(err)) => return Err(err),
        Ok(_) => {}
    };

    Ok(())
}

fn run(image_path: &str, color_tolerance: u32, zero_point: (u32, u32)) -> PanelResult<()> {
    // crop: コマの四辺にある空白を切除する
    let mut img = image::open(image_path)?;
    let mut ranges = crop::crop(&mut img, color_tolerance, &Rgba([255, 255, 255, 255]))?;

    // ゼロ点からコマ領域外を探索
    let mut labeler = Labeler::new(&mut img, ranges, 100, (200, 615))?;
    // 孤立したコマのそれぞれについて処理
    let panels = labeler.run();
    for (index, panel) in panels.iter().enumerate() {
        panel.save(&format!("panel_{}.png", index)).unwrap();
    }

    Ok(())
}
