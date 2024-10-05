use crate::color::Color;
use crate::texture::Texture;

#[derive(Clone, Debug)]
pub struct Material {
    pub color: Color,
    pub shininess: f32,
    pub properties: [f32; 4],
    pub refractive_index: f32,
    pub texture: Option<Texture>,
}

impl Material {
    pub fn new(color: Color, shininess: f32, properties: [f32; 4], refractive_index: f32) -> Self {
        Material {
            color,
            shininess,
            properties,
            refractive_index,
            texture: None,
        }
    }
 
    // Method to create a black material with default values
    pub fn black() -> Self {
        Material {
            color: Color::new(0, 0, 0),    // Use integer values for Color
            shininess: 0.0,                 // Default shininess
            properties: [0.0, 0.0, 0.0, 0.0], // Default properties (all set to 0)
            refractive_index: 1.0, 
            texture: None,         // Default refractive index (e.g., for air)
        }
    }

    pub fn with_texture(texture: Texture, shininess: f32, properties: [f32; 4], refractive_index: f32) -> Self {
        Material {
            color: Color::new(255, 255, 255), // Color base blanco
            shininess,
            properties,
            refractive_index,
            texture: Some(texture),
        }
    }

    // Method to determine if the material is completely diffuse (no shininess)
    pub fn is_diffuse(&self) -> bool {
        self.properties[1] == 0.0 && self.properties[2] == 0.0
    }

    // Method to determine if the material is reflective
    pub fn is_reflective(&self) -> bool {
        self.properties[2] > 0.0
    }

    // Method to determine if the material is transparent
    pub fn is_transparent(&self) -> bool {
        self.properties[3] > 0.0
    }
}