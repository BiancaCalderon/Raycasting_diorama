use image::{DynamicImage, GenericImageView, Rgba};
use crate::color::Color;

#[derive(Clone, Debug)]
pub struct Texture {
    image: DynamicImage,
    width: u32,
    height: u32,
}

impl Texture {
    pub fn new(file_path: &str) -> Result<Texture, image::ImageError> {
        let img = image::open(file_path)?;
        let (width, height) = img.dimensions();
        Ok(Texture { image: img, width, height })
    }

    pub fn get_color(&self, u: f32, v: f32) -> Color {
        // Convertir UV a coordenadas de píxeles
        let x = ((u % 1.0) * self.width as f32) as u32;
        let y = ((1.0 - (v % 1.0)) * self.height as f32) as u32;
        
        // Asegurar que las coordenadas estén dentro de los límites
        let x = x.clamp(0, self.width - 1);
        let y = y.clamp(0, self.height - 1);
        
        let pixel = self.image.get_pixel(x, y);
        let Rgba([r, g, b, _a]) = pixel;

        Color::new(r, g, b)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}