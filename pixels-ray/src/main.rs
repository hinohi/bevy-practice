use bevy::prelude::*;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    window::WindowResolution,
};
use bevy_pixels::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use ray_tracing::{vec3, Camera, Color, Hit, HitPoint, Material, Sphere, Vector};

#[derive(Resource)]
struct Random(StdRng);

#[derive(Resource)]
struct World {
    camera: ray_tracing::FiniteApertureCamera,
    objects: Vec<Object>,
}

struct Object {
    sphere: Sphere,
    material: Material,
}

impl World {
    fn from_rng<R: Rng>(rng: &mut R, aspect_ratio: f64) -> World {
        let camera = ray_tracing::CameraBuilder::new()
            .look_from(vec3!(13.0, 2.0, 3.0))
            .loot_at(vec3!(0.0))
            .vertical_field_of_view(20.0)
            .aspect_ratio(aspect_ratio)
            .blur(0.1);

        let mut objects = Vec::new();
        objects.push(Object {
            sphere: Sphere::new(vec3!(0.0, -1000.0, 0.0), 1000.0),
            material: Material::Lambertian {
                color: Color::new(0.5, 0.5, 0.5),
            },
        });
        for a in -11..11 {
            for b in -11..11 {
                let center = vec3!(
                    a as f64 + rng.gen_range(0.0..0.9),
                    0.2,
                    b as f64 + rng.gen_range(0.0..0.9)
                );
                if (center - vec3!(4.0, 0.2, 0.0)).norm() < 0.9 {
                    continue;
                }
                let r = rng.gen_range(0.0..1.0);
                let material = if r < 0.8 {
                    let color = rng.gen::<Vector>() * rng.gen::<Vector>().powf(2.0);
                    Material::Lambertian { color }
                } else if r < 0.95 {
                    let color = rng.gen::<Vector>() * 0.5 + 0.5;
                    let fuzz = rng.gen_range(0.0..0.5);
                    Material::Metal { color, fuzz }
                } else {
                    Material::Dielectric {
                        index_of_refraction: 1.5,
                    }
                };
                objects.push(Object {
                    sphere: Sphere::new(center, 0.2),
                    material,
                });
            }
        }
        objects.push(Object {
            sphere: Sphere::new(vec3!(0.0, 1.0, 0.0), 1.0),
            material: Material::Dielectric {
                index_of_refraction: 1.5,
            },
        });
        objects.push(Object {
            sphere: Sphere::new(vec3!(-4.0, 1.0, 0.0), 1.0),
            material: Material::Lambertian {
                color: Color::new(0.4, 0.2, 0.1),
            },
        });
        objects.push(Object {
            sphere: Sphere::new(vec3!(4.0, 1.0, 0.0), 1.0),
            material: Material::Metal {
                color: Color::new(0.7, 0.6, 0.5),
                fuzz: 0.0,
            },
        });
        World { camera, objects }
    }

    fn get_ray<R: Rng>(
        &self,
        rng: &mut R,
        width: u32,
        height: u32,
    ) -> (u32, u32, ray_tracing::Ray) {
        let x = rng.gen_range(0..width);
        let y = rng.gen_range(0..height);
        let u = (x as f64 + rng.gen::<f64>()) / width as f64;
        let v = (y as f64 + rng.gen::<f64>()) / height as f64;
        (x, y, self.camera.get_ray(rng, u, v))
    }

    fn ray_color<R: Rng>(&self, rng: &mut R, ray: &ray_tracing::Ray, depth: u32) -> Color {
        if depth == 0 {
            return ray_tracing::BLACK;
        }
        let mut t_max = f64::INFINITY;
        let mut hit: Option<(HitPoint, &Material)> = None;
        for o in self.objects.iter() {
            if let Some(new_hit) = o.sphere.hit(ray, t_max) {
                if !matches!(hit, Some((ref now_hit, _)) if now_hit.t <= new_hit.t) {
                    t_max = new_hit.t;
                    hit = Some((new_hit, &o.material));
                }
            }
        }
        if let Some((hit, mate)) = hit {
            if let Some((ray, attenuation)) = mate.scatter(rng, ray, &hit) {
                attenuation * self.ray_color(rng, &ray, depth - 1)
            } else {
                ray_tracing::BLACK
            }
        } else {
            ray.background()
        }
    }
}

#[derive(Resource)]
struct Pixels {
    width: u32,
    height: u32,
    pixels: Vec<(Color, u32)>,
}

impl Pixels {
    fn new(width: u32, height: u32) -> Pixels {
        Pixels {
            width,
            height,
            pixels: vec![(ray_tracing::BLACK, 0); (width * height) as usize],
        }
    }

    fn add_color(&mut self, x: u32, y: u32, color: Color) {
        let i = (y * self.width + x) as usize;
        self.pixels[i].0 += color;
        self.pixels[i].1 += 1;
    }

    fn iter(&self) -> impl Iterator<Item = Color> + '_ {
        self.pixels.iter().map(|&(color, count)| {
            if count > 0 {
                color / count as f64
            } else {
                ray_tracing::BLACK
            }
        })
    }
}

const WIDTH: u32 = 600;
const HEIGHT: u32 = 400;
const SCALE_FACTOR: f32 = 2.0;

fn main() {
    let mut rng = StdRng::from_entropy();
    let world = World::from_rng(&mut rng, WIDTH as f64 / HEIGHT as f64);
    App::new()
        .insert_resource(Random(rng))
        .insert_resource(world)
        .insert_resource(Pixels::new(WIDTH, HEIGHT))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ray".to_string(),
                resizable: false,
                resolution: WindowResolution::new(
                    WIDTH as f32 * SCALE_FACTOR,
                    HEIGHT as f32 * SCALE_FACTOR,
                ),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(PixelsPlugin {
            primary_window: Some(PixelsOptions {
                width: WIDTH,
                height: HEIGHT,
                scale_factor: SCALE_FACTOR,
                ..default()
            }),
        })
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(PostUpdate, draw.in_set(PixelsSet::Draw))
        .run();
}

fn draw(
    mut buffer: Query<&mut PixelsWrapper>,
    mut rng: ResMut<Random>,
    world: Res<World>,
    mut pixels: ResMut<Pixels>,
) {
    let Ok(mut wrapper) = buffer.get_single_mut() else { return };
    assert_eq!(wrapper.pixels.frame().len(), pixels.pixels.len() * 4);
    for _ in 0..10000 {
        let (x, y, ray) = world.get_ray(&mut rng.0, pixels.width, pixels.height);
        let color = world.ray_color(&mut rng.0, &ray, 50);
        let y = pixels.height - 1 - y;
        pixels.add_color(x, y, color);
    }
    let frame = wrapper.pixels.frame_mut();
    for (i, color) in pixels.iter().enumerate() {
        frame[i * 4 + 0] = (color.x() * 255.0).clamp(0.0, 255.0) as u8;
        frame[i * 4 + 1] = (color.y() * 255.0).clamp(0.0, 255.0) as u8;
        frame[i * 4 + 2] = (color.z() * 255.0).clamp(0.0, 255.0) as u8;
        frame[i * 4 + 3] = 255;
    }
}
