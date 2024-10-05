# Diorama de Minecraft con Ciclo de Día

Este proyecto es un raytracer implementado en Rust que simula un portal de Minecraft con un ciclo día/noche dinámico.

## Características

- Renderizado de escena 3D utilizando raytracing
- Ciclo día/noche dinámico con iluminación cambiante
- Materiales con texturas y propiedades físicas (reflexión, refracción, etc.)
- Objetos 3D (cubos) con diferentes materiales
- Cámara orbital controlable por el usuario
- Sombras suaves
- Paralelización para mejorar el rendimiento

## Requisitos

- Rust
- Dependencias (especificadas en `Cargo.toml`):
  - nalgebra-glm
  - minifb
  - rayon
  - image

## Instalación

1. Clona el repositorio:
   ```
   git clone https://github.com/tu-usuario/raytracer-rust.git
   cd raytracer-rust
   ```

2. Compila el proyecto:
   ```
   cargo build --release
   ```

## Uso

Ejecuta el programa con:
```
   cargo run --release
```

### Controles

- `W`: Acercar la cámara
- `S`: Alejar la cámara
- Flechas: Orbitar la cámara alrededor de la escena
- `Esc`: Salir del programa

## Estructura del Proyecto

- `src/main.rs`: Archivo principal con la lógica del raytracer y la configuración de la escena
- `src/framebuffer.rs`: Implementación del framebuffer
- `src/ray_intersect.rs`: Lógica de intersección de rayos
- `src/color.rs`: Manejo de colores
- `src/camera.rs`: Implementación de la cámara
- `src/light.rs`: Definición de luces
- `src/material.rs`: Definición de materiales
- `src/cube.rs`: Implementación de cubos
- `src/texture.rs`: Manejo de texturas

## Personalización

Puedes modificar la escena ajustando los objetos, materiales y luces en la función `main()`. También puedes cambiar las texturas cargando nuevos archivos de imagen en la carpeta `assets/`.

Enlace del proyecto: https://github.com/tu-usuario/raytracer-rust
