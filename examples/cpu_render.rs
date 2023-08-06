use image::ImageBuffer;
use image::Rgb;

#[derive(Default, Clone, Debug, PartialEq)]
struct RGB {
    r: u8,
    g: u8,
    b: u8,
}

impl RGB {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        RGB { r, g, b }
    }
}

use shocovox_rs::spatial::math::V3c;
use shocovox_rs::spatial::Ray;
fn main() {
    // fill octree with data
    let mut tree = shocovox_rs::octree::Octree::<RGB>::new(4).ok().unwrap();
    tree.insert(&V3c::new(3, 0, 0), RGB::new(255, 0, 0)).ok();
    tree.insert(&V3c::new(3, 3, 0), RGB::new(0, 255, 0)).ok();
    tree.insert(&V3c::new(0, 3, 0), RGB::new(0, 0, 255)).ok();

    for y in 0..4 {
        tree.insert(
            &V3c::new(0, y, y),
            RGB {
                r: 64 * y as u8 + 50,
                g: 64 * y as u8 + 50,
                b: 64 * y as u8 + 50,
            },
        )
        .ok();
        tree.insert(
            &V3c::new(1, y, y),
            RGB {
                r: 255,
                g: 64 * y as u8,
                b: 64 * y as u8,
            },
        )
        .ok();
        tree.insert(
            &V3c::new(2, y, y),
            RGB {
                r: 64 * y as u8,
                g: 255,
                b: 64 * y as u8,
            },
        )
        .ok();
        tree.insert(
            &V3c::new(3, y, y),
            RGB {
                r: 64 * y as u8,
                g: 64 * y as u8,
                b: 255,
            },
        )
        .ok();
    }

    // Set the viewport
    let viewport = Ray {
        origin: V3c::new(10., 10., -5.),
        direction: V3c::new(-2., -1., 1.).normalized(),
    };
    let viewport_up_direction = V3c::new(0., 1., 0.); //TODO: up is actually left?!
    let viewport_right_direction = viewport_up_direction.cross(viewport.direction).normalized();
    let viewport_width = 4.;
    let viewport_height = 4.;
    let viewport_resolution_width = 512;
    let viewport_resolution_height = 512;
    let viewport_fov = 3.;
    let pixel_width = viewport_width as f32 / viewport_resolution_width as f32;
    let pixel_height = viewport_height as f32 / viewport_resolution_height as f32;
    let viewport_bottom_left = viewport.origin + (viewport.direction * viewport_fov)
        - (viewport_up_direction * (viewport_height / 2.))
        - (viewport_right_direction * (viewport_width / 2.));

    // define light
    let diffuse_light_normal = V3c::new(0., -1., 1.).normalized();

    // cast each ray for a hit
    let mut img = ImageBuffer::new(viewport_resolution_width, viewport_resolution_height);

    for y in 0..viewport_resolution_width {
        for x in 0..viewport_resolution_height {
            let actual_y_in_image = viewport_resolution_height - y - 1;
            //from the origin of the camera to the current point of the viewport
            let glass_point = viewport_bottom_left
                + viewport_right_direction * x as f32 * pixel_width
                + viewport_up_direction * y as f32 * pixel_height;
            let ray = Ray {
                origin: viewport.origin,
                direction: (glass_point - viewport.origin).normalized(),
            };
            if let Some(hit) = tree.get_by_ray(&ray) {
                let (data, _, normal) = hit;
                //Because both vector should be normalized, the dot product should be 1*1*cos(angle)
                //That means it is in range -1, +1, which should be accounted for
                let diffuse_light_strength = 1. - (normal.dot(&diffuse_light_normal) / 2. + 0.5);
                img.put_pixel(
                    x,
                    actual_y_in_image,
                    Rgb([
                        (data.r as f32 * diffuse_light_strength) as u8,
                        (data.g as f32 * diffuse_light_strength) as u8,
                        (data.b as f32 * diffuse_light_strength) as u8,
                    ]),
                );
            } else {
                img.put_pixel(x, actual_y_in_image, Rgb([0, 0, 0]));
            }
            print!(
                "\r   progress: {}   ",
                (x + y * viewport_resolution_height) as f32
                    / (viewport_resolution_height * viewport_resolution_width) as f32
            );
        }
    }

    // Display results //TODO: do it in a more sophisticated way
    println!("Done!");
    img.save("render.png").unwrap();
}
