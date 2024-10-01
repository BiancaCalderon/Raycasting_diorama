use image::{GenericImageView, RgbaImage};
use crate::color::Color;

pub struct Texture {
    pub image: RgbaImage,
}

impl Texture {
    pub fn new(file_path: &str) -> Self {
        let img = image::open(file_path).expect("Failed to load texture image");
        let (width, height) = img.dimensions();
        let rgba_image = img.to_rgba8(); // Convierte a RgbaImage
        Texture {
            image: rgba_image,
        }
    }

    pub fn get_color(&self, u: f32, v: f32) -> Color {
        let width = self.image.width() as f32;
        let height = self.image.height() as f32;

        // Convertir coordenadas UV a píxeles
        let x = (u * width).clamp(0.0, width - 1.0) as u32;
        let y = (v * height).clamp(0.0, height - 1.0) as u32;

        let pixel = self.image.get_pixel(x, y);
        Color::new(pixel[0], pixel[1], pixel[2]) // Asumiendo que Color tiene un constructor que acepta RGB
    }
}
