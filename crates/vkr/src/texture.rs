// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use crate::*;

pub struct Texture {
    pub view: Handle<ImageView>,
    pub sampler: Handle<Sampler>,
}

impl Texture {
    pub fn new(view: Handle<ImageView>, sampler: Handle<Sampler>) -> Self {
        Self { view, sampler }
    }
}

#[cfg(test)]
mod test {
    use std::{
        fs::{create_dir, File},
        io::BufWriter,
        path::Path,
    };

    #[test]
    fn save_png() {
        let image_dir = Path::new(r"res/image");
        if !Path::exists(&image_dir) {
            create_dir(image_dir).expect("Failed to create image directory");
        }

        let path = Path::new(r"res/image/test.png");
        let file = File::create(path).unwrap();
        let ref mut w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, 2, 2);
        encoder.set_color(png::ColorType::RGBA);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header().unwrap();
        // 4 pixels
        let data = [
            180, 100, 10, 255, 20, 190, 10, 205, 40, 10, 200, 255, 80, 100, 200, 255,
        ];
        writer.write_image_data(&data).unwrap();
    }

    #[test]
    fn test_copy_image() {
        // TODO a CTX without any window
    }
}
