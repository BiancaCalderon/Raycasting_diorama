use nalgebra_glm::{Vec3, normalize};
use minifb::{Key, Window, WindowOptions};
use std::time::Duration;
use std::f32::consts::PI;
use rayon::prelude::*;

mod framebuffer;
mod ray_intersect;
mod color;
mod camera;
mod light;
mod material;
mod cube;
mod texture;

use framebuffer::Framebuffer;
use color::Color;
use ray_intersect::{Intersect, RayIntersect};
use camera::Camera;
use light::Light;
use crate::cube::Cube;
use crate::material::Material;
use texture::Texture;

extern crate image;

const ORIGIN_BIAS: f32 = 1e-4;
const SKYBOX_COLOR: Color = Color::new(68, 142, 228);

// Añade estas constantes
const DAY_DURATION: f32 = 10.0; // Duración del día en segundos
const NIGHT_SKY_COLOR: Color = Color::new(10, 10, 50); // Color del cielo nocturno

// Modifica la estructura Light para incluir el ciclo día/noche
struct SceneLight {
    position: Vec3,
    color: Color,
    intensity: f32,
    time: f32,
}

impl SceneLight {
    fn new(position: Vec3, color: Color, intensity: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            time: 0.0,
        }
    }

    fn update(&mut self, delta_time: f32) {
        self.time += delta_time;
        if self.time > DAY_DURATION {
            self.time -= DAY_DURATION;
        }

        let angle = 2.0 * PI * (self.time / DAY_DURATION);
        
        // Actualiza la posición de la luz
        self.position = Vec3::new(
            0.75 * angle.cos(),
            0.25 + 2.0 * angle.sin(),
            -2.0
        );

        // Actualiza el color y la intensidad de la luz
        let t = (angle.sin() + 1.0) / 2.0; // Normaliza entre 0 y 1
        self.color = Color::new(
            (255.0 * t) as u8,
            (200.0 * t) as u8,
            (100.0 * t) as u8
        );
        self.intensity = 1.0 + t;
    }
}

fn offset_origin(intersect: &Intersect, direction: &Vec3) -> Vec3 {
    let offset = intersect.normal * ORIGIN_BIAS;
    if direction.dot(&intersect.normal) < 0.0 {
        intersect.point - offset
    } else {
        intersect.point + offset
    }
}

fn reflect(incident: &Vec3, normal: &Vec3) -> Vec3 {
    incident - 2.0 * incident.dot(normal) * normal
}

fn refract(incident: &Vec3, normal: &Vec3, eta_t: f32) -> Vec3 {
    let cosi = -incident.dot(normal).max(-1.0).min(1.0);
    
    let (n_cosi, eta, n_normal);

    if cosi < 0.0 {
        // Ray is entering the object
        n_cosi = -cosi;
        eta = 1.0 / eta_t;
        n_normal = -normal;
    } else {
        // Ray is leaving the object
        n_cosi = cosi;
        eta = eta_t;
        n_normal = *normal;
    }
    
    let k = 1.0 - eta * eta * (1.0 - n_cosi * n_cosi);
    
    if k < 0.0 {
        // Total internal reflection
        reflect(incident, &n_normal)
    } else {
        eta * incident + (eta * n_cosi - k.sqrt()) * n_normal
    }
}

fn cast_shadow(
    intersect: &Intersect,
    light: &SceneLight,
    objects: &[Cube],
) -> f32 {
    let light_dir = (light.position - intersect.point).normalize();
    let light_distance = (light.position - intersect.point).magnitude();

    let shadow_ray_origin = offset_origin(intersect, &light_dir);
    let mut shadow_intensity = 0.0;

    for object in objects {
        let shadow_intersect = object.ray_intersect(&shadow_ray_origin, &light_dir);
        if shadow_intersect.is_intersecting && shadow_intersect.distance < light_distance {
            let distance_ratio = shadow_intersect.distance / light_distance;
            shadow_intensity = 1.0 - distance_ratio.powf(2.0).min(1.0);
            break;
        }
    }

    shadow_intensity
}

// Modifica la función cast_ray para usar el color del cielo variable
fn cast_ray(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Cube],
    light: &SceneLight,
    depth: u32,
    sky_color: Color,
) -> Color {
    if depth > 3 {
        return sky_color;
    }

    let mut intersect = Intersect::empty();
    let mut zbuffer = f32::INFINITY;

    for object in objects {
        let i = object.ray_intersect(ray_origin, ray_direction);
        if i.is_intersecting && i.distance < zbuffer {
            zbuffer = i.distance;
            intersect = i;
        }
    }

    if !intersect.is_intersecting {
        return sky_color;
    }

    // Añadir la emisión del material al color base
    let emission = intersect.material.emission;

    fn calculate_uv(intersect: &Intersect) -> (f64, f64) {
        // Determinar qué cara del cubo estamos renderizando
        let normal = intersect.normal;
        let point = intersect.point;

        let (u, v) = if normal.y.abs() > 0.99 {
            // Cara superior o inferior
            (point.x.abs() % 1.0, point.z.abs() % 1.0)
        } else if normal.x.abs() > 0.99 {
            // Cara lateral (izquierda o derecha)
            (point.z.abs() % 1.0, point.y.abs() % 1.0)
        } else {
            // Cara frontal o trasera
            (point.x.abs() % 1.0, point.y.abs() % 1.0)
        };

        (u as f64, v as f64)
    }
    

    let material_color = if let Some(texture) = &intersect.material.texture {
        let uv = calculate_uv(&intersect);
        let u = uv.0.fract();
        let v = uv.1.fract();
        texture.get_color(u as f32, v as f32)
    } else {
        intersect.material.color
    };
    
    // Intensity of the light hitting the object
    let light_dir = (light.position - intersect.point).normalize();
    let view_dir = (ray_origin - intersect.point).normalize();
    let reflect_dir = reflect(&-light_dir, &intersect.normal).normalize();
    
    let shadow_intensity = cast_shadow(&intersect, light, objects);
    let light_intensity = light.intensity * (1.0 - shadow_intensity);
    
    // Determinar si el material tiene una textura
    let has_texture = intersect.material.texture.is_some();

    // Calcular el color base
    let base_color = if has_texture {
        material_color + emission // Añadir emisión
    } else {
        // Aplicar iluminación solo para materiales sin textura
        let diffuse_intensity = intersect.normal.dot(&light_dir).max(0.0).min(1.0);
        let diffuse = Color::black() * intersect.material.properties[0] * diffuse_intensity * light_intensity;
        
        let specular_intensity = view_dir.dot(&reflect_dir).max(0.0).powf(intersect.material.shininess);
        let specular = light.color * intersect.material.properties[1] * specular_intensity * light_intensity;
        
        diffuse + specular + emission // Añadir emisión
    };

    // Reflected color
    let mut reflect_color = Color::black();
    let reflectivity = intersect.material.properties[2];
    if reflectivity > 0.0 {
        let reflect_dir = reflect(&ray_direction, &intersect.normal).normalize();
        let reflect_origin = offset_origin(&intersect, &reflect_dir);
        reflect_color = cast_ray(&reflect_origin, &reflect_dir, objects, light, depth + 1, sky_color);
    }
    
    // Refracted color
    let mut refract_color = Color::black();
    let transparency = intersect.material.properties[3];
    if transparency > 0.0 {
        let refract_dir = refract(&ray_direction, &intersect.normal, intersect.material.refractive_index);
        let refract_origin = offset_origin(&intersect, &refract_dir);
        refract_color = cast_ray(&refract_origin, &refract_dir, objects, light, depth + 1, sky_color);
    }
    
    // Combinar los colores
    if has_texture {
        base_color * (1.0 - reflectivity - transparency) + (reflect_color * reflectivity) + (refract_color * transparency)
    } else {
        base_color + (reflect_color * reflectivity) + (refract_color * transparency)
    }

}

// Modifica la función render para pasar el color del cielo
pub fn render(framebuffer: &mut Framebuffer, objects: &[Cube], camera: &Camera, light: &SceneLight, sky_color: Color) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let fov = PI / 3.0;
    let perspective_scale = (fov * 0.5).tan();


    // Crea un búfer temporal para almacenar los colores de los píxeles
    let mut pixel_buffer = vec![0u32; (framebuffer.width * framebuffer.height) as usize];


    // Utiliza paralelización para calcular los colores
    pixel_buffer
        .par_iter_mut()  // Iterador paralelo sobre el búfer
        .enumerate()
        .for_each(|(index, pixel)| {
            let x = (index % framebuffer.width as usize) as u32;
            let y = (index / framebuffer.width as usize) as u32;


            let screen_x = (2.0 * x as f32) / width - 1.0;
            let screen_y = -(2.0 * y as f32) / height + 1.0;


            let screen_x = screen_x * aspect_ratio * perspective_scale;
            let screen_y = screen_y * perspective_scale;


            let ray_direction = normalize(&Vec3::new(screen_x, screen_y, -1.0));
            let rotated_direction = camera.basis_change(&ray_direction);


            let pixel_color = cast_ray(&camera.eye, &rotated_direction, objects, light, 0, sky_color);


            // Asigna el color calculado en el buffer de píxeles
            *pixel = pixel_color.to_hex();
        });


    // Finalmente, vuelca el pixel_buffer en el framebuffer
    for (index, &pixel) in pixel_buffer.iter().enumerate() {
        let x = (index % framebuffer.width as usize) as u32;
        let y = (index / framebuffer.width as usize) as u32;
        framebuffer.set_current_color(pixel);
        framebuffer.point(x as usize, y as usize);
    }
}

fn main() {
    let window_width = 800;
    let window_height = 600;
    let framebuffer_width = 800;
    let framebuffer_height = 600;
    let frame_delay = Duration::from_millis(16);

    let mut framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);
    let mut window = Window::new(
        "Rust Graphics - Raytracer Example",
        window_width,
        window_height,
        WindowOptions::default(),
    ).unwrap();

    // move the window around
    window.set_position(500, 500);
    window.update();

    fn load_texture(file_path: &str) -> Texture {
        match Texture::new(file_path) {
            Ok(texture) => texture,
            Err(e) => {
                eprintln!("Error al cargar la textura {}: {}", file_path, e);
                // Aquí podrías devolver una textura por defecto o pánico, dependiendo de tus necesidades
                panic!("No se pudo cargar la textura");
            }
        }
    }

    let obsidian_texture  = load_texture("assets/obsidian.jpg"); // Carga la textura de obsidiana
    let purple_texture  = load_texture("assets/purple.jpg"); // Carga la textura púrpura
    let grass_texture = load_texture("assets/grass.jpg");
    //let lava_texture = load_texture("assets/lava.jpg");

    let obsidian_material = Material::with_texture(
        obsidian_texture, // Texture para obsidian
        10.0,            // Brillo
        [0.1, 0.9, 0.1, 0.0], // Propiedades
        2.0               // Índice de refracción
    );

    let purple_material = Material::with_texture(
        purple_texture,   // Texture para purple
        10.0,            // Brillo
        [0.1, 0.9, 0.1, 0.0], // Propiedades
        1.0               // Índice de refracción
    );


    // Define el material de césped
    let grass = Material::with_texture(
        grass_texture,  // Color verde
        10.0,                   // Ajuste el brillo si es necesario
        [0.8, 0.2, 0.0, 0.0],   // Ajusta las propiedades: difuso, especular, reflectividad, transparencia
        1.0
    );

    // Material para rock
    let rock: Material = Material::new(
        Color::new(169, 169, 169), // Color gris (Rocoso)
        100.0,                      // Ajuste el brillo
        [0.6, 0.6, 0.6, 0.0],      // Propiedades: difuso, especular, reflectividad, transparencia
        0.0
    );

    // Material para lava
    let lava_texture = match Texture::new("assets/lava.jpg") {
        Ok(texture) => texture,
        Err(e) => {
            eprintln!("Error al cargar la textura de lava: {}", e);
            panic!("No se pudo cargar la textura de lava");
        }
    };

    let mut lava = Material::with_texture(
        lava_texture,
        0.0,                // shininess (brillo)
        [0.9, 0.3, 0.0, 0.5], // propiedades: difuso, especular, reflectividad, transparencia
        1.0                 // índice de refracción
    );

    // Añadir emisión al material de lava
    lava.emission = Color::new(255, 128, 0); // Color de emisión naranja (usando valores u8)

    
    // Ajustar la luz
    let mut light = SceneLight::new(Vec3::new(0.75, 0.25, -2.0), Color::new(255, 200, 100), 2.0);

    let delta_y = 0.703125; // Aumentado un 25% adicional
    let delta_z = 0.46875;  // Aumentado un 25% adicional

    let objects = [
        // Base con césped (aumentada)
        Cube { min: Vec3::new(-1.40625, -0.234375, -1.40625), max: Vec3::new(1.40625, -0.09375, 1.40625), material: grass },

        // Lava en las esquinas de la base (aumentada)
        Cube { min: Vec3::new(-1.5, -0.234375, -1.5), max: Vec3::new(-1.3125, 0.0, -1.3125), material: lava.clone() },
        Cube { min: Vec3::new(1.3125, -0.234375, -1.5), max: Vec3::new(1.5, 0.0, -1.3125), material: lava.clone() },
        Cube { min: Vec3::new(-1.5, -0.234375, 1.3125), max: Vec3::new(-1.3125, 0.0, 1.5), material: lava.clone() },
        Cube { min: Vec3::new(1.3125, -0.234375, 1.3125), max: Vec3::new(1.5, 0.0, 1.5), material: lava.clone() },

        // Portal (marco)
        Cube { min: Vec3::new(-0.46875, 0.09375 + delta_y, -0.703125 + delta_z), max: Vec3::new(-0.234375, 1.171875 + delta_y, -0.234375 + delta_z), material: obsidian_material.clone() },
        Cube { min: Vec3::new(0.234375, 0.09375 + delta_y, -0.703125 + delta_z), max: Vec3::new(0.46875, 1.171875 + delta_y, -0.234375 + delta_z), material: obsidian_material.clone() },
        Cube { min: Vec3::new(-0.46875, 1.171875 + delta_y, -0.703125 + delta_z), max: Vec3::new(0.46875, 1.40625 + delta_y, -0.234375 + delta_z), material: obsidian_material.clone() },
        Cube { min: Vec3::new(-0.46875, -0.09375 + delta_y, -0.703125 + delta_z), max: Vec3::new(0.46875, 0.09375 + delta_y, -0.234375 + delta_z), material: obsidian_material.clone() },

        // Columnas del portal
        Cube { 
            min: Vec3::new(-0.234375, 0.09375 + delta_y, -0.703125 + delta_z), 
            max: Vec3::new(0.0, 1.171875 + delta_y, -0.234375 + delta_z), 
            material: purple_material.clone() 
        },
        Cube { 
            min: Vec3::new(0.0, 0.09375 + delta_y, -0.703125 + delta_z), 
            max: Vec3::new(0.234375, 1.171875 + delta_y, -0.234375 + delta_z), 
            material: purple_material 
        },

        // Gradas
        Cube { min: Vec3::new(-1.125, -0.140625, -1.125), max: Vec3::new(1.125, -0.046875, 1.453125), material: rock.clone() },
        Cube { min: Vec3::new(-1.078125, -0.046875, -1.078125), max: Vec3::new(1.078125, 0.046875, 1.359375), material: rock.clone() }, 
        Cube { min: Vec3::new(-1.03125, 0.046875, -1.03125), max: Vec3::new(1.03125, 0.140625, 1.265625), material: rock.clone() },  
        Cube { min: Vec3::new(-0.984375, 0.140625, -0.984375), max: Vec3::new(0.984375, 0.234375, 1.171875), material: rock.clone() },  
        Cube { min: Vec3::new(-0.9375, 0.234375, -0.9375), max: Vec3::new(0.9375, 0.328125, 1.078125), material: rock.clone() }, 
        Cube { min: Vec3::new(-0.890625, 0.328125, -0.890625), max: Vec3::new(0.890625, 0.421875, 0.984375), material: rock.clone() },  
        Cube { min: Vec3::new(-0.84375, 0.421875, -0.84375), max: Vec3::new(0.84375, 0.515625, 0.890625), material: rock.clone() }, 
        Cube { min: Vec3::new(-0.796875, 0.515625, -0.796875), max: Vec3::new(0.796875, 0.609375, 0.75), material: rock.clone() },  
    ];

    // Inicializa la cámara con una posición más lejana para compensar el aumento de tamaño
    let mut camera = Camera::new(
        Vec3::new(0.0, 0.0, 5.5),
        Vec3::new(0.0, 0.0, 0.0),  // punto al que la cámara está mirando (origen)
        Vec3::new(0.0, 1.0, 0.0)   // vector hacia arriba del mundo
    );

    let rotation_speed = PI / 50.0;

    const ZOOM_SPEED: f32 = 0.05;  // Reducido para un control más fino

    let mut last_update = std::time::Instant::now();

    while window.is_open() {
        // Escuchar entradas
        if window.is_key_down(Key::Escape) {
            break;
        }

        // Si presionas la tecla W, la cámara se acerca
        if window.is_key_down(Key::W) {
            let forward = (camera.center - camera.eye).normalize();
            camera.eye += forward * ZOOM_SPEED;
        }

        // Si presionas la tecla S, la cámara se aleja
        if window.is_key_down(Key::S) {
            let backward = (camera.eye - camera.center).normalize();
            camera.eye += backward * ZOOM_SPEED;
        }

        // Controles de órbita de la cámara
        if window.is_key_down(Key::Left) {
            camera.orbit(rotation_speed, 0.0);
        }
        if window.is_key_down(Key::Right) {
            camera.orbit(-rotation_speed, 0.0);
        }
        if window.is_key_down(Key::Up) {
            camera.orbit(0.0, -rotation_speed);
        }
        if window.is_key_down(Key::Down) {
            camera.orbit(0.0, rotation_speed);
        }

        // Actualiza la luz y calcula el color del cielo
        let now = std::time::Instant::now();
        let delta_time = (now - last_update).as_secs_f32();
        last_update = now;

        light.update(delta_time);

        let t = (light.position.y + 2.0) / 4.0; // Normaliza entre 0 y 1
        let sky_color = Color::new(
            (SKYBOX_COLOR.red() as f32 * t + NIGHT_SKY_COLOR.red() as f32 * (1.0 - t)) as u8,
            (SKYBOX_COLOR.green() as f32 * t + NIGHT_SKY_COLOR.green() as f32 * (1.0 - t)) as u8,
            (SKYBOX_COLOR.blue() as f32 * t + NIGHT_SKY_COLOR.blue() as f32 * (1.0 - t)) as u8,
        );

        // Dibuja los objetos con el nuevo color del cielo
        render(&mut framebuffer, &objects, &camera, &light, sky_color);

        // Actualiza la ventana con el contenido del framebuffer
        window
            .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }
}