use crate::color::Color;
use crate::texture::Texture;

#[derive(Clone, Debug)]
pub struct Material {
    pub color: Color,
    pub texture: Option<Texture>,
    pub shininess: f32,
    pub properties: [f32; 4],
    pub refractive_index: f32,
    pub emission: Color, // Nueva propiedad para la emisividad
}

impl Material {
    pub fn new(color: Color, shininess: f32, properties: [f32; 4], refractive_index: f32) -> Self {
        Material {
            color,
            texture: None,
            shininess,
            properties,
            refractive_index,
            emission: Color::black(), // Por defecto, no emite luz
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
            emission: Color::black(), // Por defecto, no emite luz
        }
    }

    pub fn with_texture(texture: Texture, shininess: f32, properties: [f32; 4], refractive_index: f32) -> Self {
        Material {
            color: Color::white(),
            texture: Some(texture),
            shininess,
            properties,
            refractive_index,
            emission: Color::black(), // Por defecto, no emite luz
        }
    }

    // Nuevo método para crear materiales emisivos
    pub fn with_emission(color: Color, shininess: f32, properties: [f32; 4], refractive_index: f32, emission: Color) -> Self {
        Material {
            color,
            texture: None,
            shininess,
            properties,
            refractive_index,
            emission,
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