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

const ORIGIN_BIAS: f32 = 1e-4;
const SKYBOX_COLOR: Color = Color::new(68, 142, 228);

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
    light: &Light,
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

pub fn cast_ray(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Cube],
    light: &Light,
    depth: u32,
) -> Color {
    if depth > 3 {
        return SKYBOX_COLOR;
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
        return SKYBOX_COLOR;
    }

    let light_dir = (light.position - intersect.point).normalize();
    let view_dir = (ray_origin - intersect.point).normalize();
    let reflect_dir = reflect(&-light_dir, &intersect.normal).normalize();

    // Calcula la intensidad de la sombra
    let shadow_intensity = cast_shadow(&intersect, light, objects);
    let light_intensity = light.intensity * (1.0 - shadow_intensity);

    // Intensidad difusa
    let diffuse_intensity = intersect.normal.dot(&light_dir).max(0.0).min(1.0);
    let diffuse = intersect.material.color * intersect.material.properties[0] * diffuse_intensity * light_intensity;

    // Intensidad especular
    let specular_intensity = view_dir.dot(&reflect_dir).max(0.0).powf(intersect.material.shininess);
    let specular = light.color * intersect.material.properties[1] * specular_intensity * light_intensity;

    // Color reflejado
    let mut reflect_color = Color::black();
    let reflectivity = intersect.material.properties[2];
    if reflectivity > 0.0 {
        let reflect_dir = reflect(&ray_direction, &intersect.normal).normalize();
        let reflect_origin = offset_origin(&intersect, &reflect_dir);
        reflect_color = cast_ray(&reflect_origin, &reflect_dir, objects, light, depth + 1);
    }

    // Color refractado
    let mut refract_color = Color::black();
    let transparency = intersect.material.properties[3];
    if transparency > 0.0 {
        let refract_dir = refract(&ray_direction, &intersect.normal, intersect.material.refractive_index);
        let refract_origin = offset_origin(&intersect, &refract_dir);
        refract_color = cast_ray(&refract_origin, &refract_dir, objects, light, depth + 1);
    }

    // Combinación de los colores difuso, especular, reflejado y refractado
    (diffuse + specular) * (1.0 - reflectivity - transparency) + (reflect_color * reflectivity) + (refract_color * transparency)
}


pub fn render(framebuffer: &mut Framebuffer, objects: &[Cube], camera: &Camera, light: &Light) {
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


            let pixel_color = cast_ray(&camera.eye, &rotated_direction, objects, light, 0);


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

    let light = Light::new(
        Vec3::new(1.0, 1.0, 5.0),
        Color::new(255, 255, 255),
        1.0
    );

    // Define el material de césped
    const GRASS: Material = Material::new(
        Color::new(0, 255, 0),  // Color verde
        50.0,                   // Ajuste el brillo si es necesario
        [0.8, 0.2, 0.0, 0.0],   // Ajusta las propiedades: difuso, especular, reflectividad, transparencia
        1.0
    );

    // Material para Obsidian
    const OBSIDIAN: Material = Material::new(
        Color::new(0, 0, 0),     // Color negro
        100.0,                   // Ajuste el brillo (puede ser más alto para reflejar más luz)
        [0.1, 0.9, 0.1, 0.0],    // Propiedades: difuso, especular, reflectividad, transparencia
        1.0
    );

    // Material para Purple
    const PURPLE: Material = Material::new(
        Color::new(128, 0, 128),     // Color morado
        100.0,                        // Ajuste el brillo
        [0.6, 0.9, 0.6, 0.0],        // Propiedades: difuso, especular, reflectividad, transparencia
        2.0
    );

    // Material para ROCK
    const ROCK: Material = Material::new(
        Color::new(169, 169, 169), // Color gris (Rocoso)
        50.0,                      // Ajuste el brillo
        [0.6, 0.6, 0.6, 0.0],      // Propiedades: difuso, especular, reflectividad, transparencia
        0.0
    );

    // Material para LAVA
    const LAVA: Material = Material::new(
        Color::new(255, 69, 0), // Color naranja brillante (Lava)
        100.0,                   // Ajuste el brillo
        [0.9, 0.3, 0.0, 0.5],   // Propiedades: difuso, especular, reflectividad, transparencia
        0.0                     // Otras propiedades si es necesario
    );

    
    // Agregar una luz
    let light = Light::new(Vec3::new(2.8, 1.0, -3.0), Color::new(255, 165, 0), 1.5);

let delta_y = 1.5; // Desplazamiento del portal hacia arriba
let delta_z = 1.0; // Desplazamiento del portal hacia adelante

let objects = [
    // Base con césped (desplazada hacia abajo)
    Cube { min: Vec3::new(-3.0, -0.5, -3.0), max: Vec3::new(3.0, -0.2, 3.0), material: GRASS }, // Base de césped

     // Lava en las esquinas de la base
     Cube { min: Vec3::new(-3.2, -0.5, -3.2), max: Vec3::new(-2.8, 0.0, -2.8), material: LAVA }, // Esquina inferior izquierda
     Cube { min: Vec3::new(2.8, -0.5, -3.2), max: Vec3::new(3.2, 0.0, -2.8), material: LAVA },  // Esquina inferior derecha
     Cube { min: Vec3::new(-3.2, -0.5, 2.8), max: Vec3::new(-2.8, 0.0, 3.2), material: LAVA },  // Esquina superior izquierda
     Cube { min: Vec3::new(2.8, -0.5, 2.8), max: Vec3::new(3.2, 0.0, 3.2), material: LAVA },   // Esquina superior derecha

     // Portal (marco) desplazado hacia arriba en delta_y y hacia adelante en delta_z
    // Lados verticales del marco
    Cube { min: Vec3::new(-1.0, 0.2 + delta_y, -1.5 + delta_z), max: Vec3::new(-0.5, 2.5 + delta_y, -0.5 + delta_z), material: OBSIDIAN }, // Izquierda
    Cube { min: Vec3::new(0.5, 0.2 + delta_y, -1.5 + delta_z), max: Vec3::new(1.0, 2.5 + delta_y, -0.5 + delta_z), material: OBSIDIAN },  // Derecha
    
    // Lados horizontales del marco
    Cube { min: Vec3::new(-1.0, 2.5 + delta_y, -1.5 + delta_z), max: Vec3::new(1.0, 3.0 + delta_y, -0.5 + delta_z), material: OBSIDIAN }, // Arriba
    Cube { min: Vec3::new(-1.0, -0.2 + delta_y, -1.5 + delta_z), max: Vec3::new(1.0, 0.2 + delta_y, -0.5 + delta_z), material: OBSIDIAN }, // Abajo

    // Interior del portal desplazado hacia arriba y hacia adelante
    Cube { 
        min: Vec3::new(-0.5, 0.2 + delta_y, -1.5 + delta_z), 
        max: Vec3::new(0.5, 2.5 + delta_y, -0.5 + delta_z), 
        material: PURPLE 
    }, // Interior morado ajustado
    

    // Gradas desde un extremo al otro de la base verde
    Cube { min: Vec3::new(-2.4, -0.3, -2.4), max: Vec3::new(2.4, -0.1, 3.1), material: ROCK },
    Cube { min: Vec3::new(-2.3, -0.1, -2.3), max: Vec3::new(2.3, 0.1, 2.9), material: ROCK },  // Tercer escalón
    Cube { min: Vec3::new(-2.2, 0.1, -2.2), max: Vec3::new(2.2, 0.3, 2.7), material: ROCK },   // Cuarto escalón
    Cube { min: Vec3::new(-2.1, 0.3, -2.1), max: Vec3::new(2.1, 0.5, 2.5), material: ROCK },   // Quinto escalón
    Cube { min: Vec3::new(-2.0, 0.5, -2.0), max: Vec3::new(2.0, 0.7, 2.3), material: ROCK },   // Sexto escalón
    Cube { min: Vec3::new(-1.9, 0.7, -1.9), max: Vec3::new(1.9, 0.9, 2.1), material: ROCK },   // Séptimo escalón
    Cube { min: Vec3::new(-1.8, 0.9, -1.8), max: Vec3::new(1.8, 1.1, 2.0), material: ROCK },   // Octavo escalón
    Cube { min: Vec3::new(-1.7, 1.1, -1.7), max: Vec3::new(1.7, 1.3, 1.8), material: ROCK },   // Noveno escalón

];

    // Inicializa la cámara
    let mut camera = Camera::new(
        Vec3::new(0.0, 0.0, 6.5),  // posición inicial de la cámara
        Vec3::new(0.0, 0.0, 0.0),  // punto al que la cámara está mirando (origen)
        Vec3::new(0.0, 1.0, 0.0)   // vector hacia arriba del mundo
    );
    let rotation_speed = PI / 50.0;
    let zoom_speed = 0.5;
    const MAX_ZOOM: f32 = 1.0;
    const MIN_ZOOM: f32 = 10.0;

    while window.is_open() {
        // Escuchar entradas
        if window.is_key_down(Key::Escape) {
            break;
        }

        // Si presionas la tecla W, la cámara se acerca
        if window.is_key_down(Key::W) {
            if camera.eye.z - zoom_speed > MAX_ZOOM {
                camera.eye.z -= zoom_speed;
            } else {
                camera.eye.z = MAX_ZOOM;
            }
        }
    
        // Si presionas la tecla S, la cámara se aleja
        if window.is_key_down(Key::S) {
            if camera.eye.z + zoom_speed < MIN_ZOOM {
                camera.eye.z += zoom_speed;
            } else {
                camera.eye.z = MIN_ZOOM;
            }
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

        // Dibuja los objetos
        render(&mut framebuffer, &objects, &camera, &light);

        // Actualiza la ventana con el contenido del framebuffer
        window
            .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }
}
