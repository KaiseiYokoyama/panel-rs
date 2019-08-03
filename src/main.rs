#![feature(test)]

extern crate test;

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
    match run("panels.png", 400, (0, 0)) {
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
    let mut cropped_img = crop::crop(&mut img, color_tolerance, zero_point)?;

    // ゼロ点からコマ領域外を探索
    irrigate::irrigate(&mut cropped_img, 100, (200, 615))?;
    // 孤立したコマのそれぞれについて処理

    Ok(())
}

#[cfg(test)]
mod tests {
    use test::Bencher;
    use crate::{crop::crop, irrigate::Irrigater};

    #[bench]
    fn crop_bench(b: &mut Bencher) {
        b.iter(|| {
            let mut img = image::open("panels.png").unwrap();
            let _cropped_img = crop(&mut img, 100, (0, 0)).unwrap();
        })
    }

    #[bench]
    fn irrigate_bench(b: &mut Bencher) {
        b.iter(|| {
            let mut img = image::open("panels.png").unwrap();
            let cropped_img = crop(&mut img, 100, (0, 0)).unwrap();
            Irrigater::new(&cropped_img, 100, (200, 615)).unwrap().flood_fill();
        });
    }
}


