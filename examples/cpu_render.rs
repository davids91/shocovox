#[cfg(feature = "raytracing")]
use rand::Rng;

#[cfg(feature = "raytracing")]
use shocovox_rs::{octree::V3c, raytracing::Ray};

#[cfg(feature = "raytracing")]
#[show_image::main]
fn main() {
    let voxel_color: Albedo = 0x645097FF.into();

    // fill octree with data
    const BRICK_DIMENSION: u32 = 8;
    const TREE_SIZE: u32 = 64;
    let viewport_size_width = 150;
    let viewport_size_height = 150;
    let mut tree = shocovox_rs::octree::BoxTree::<Albedo>::new(TREE_SIZE, BRICK_DIMENSION)
        .ok()
        .unwrap();

    tree.insert(&V3c::new(1, 3, 3), &voxel_color)
        .expect("insert of voxel to work");
    for x in 0..TREE_SIZE {
        for y in 0..TREE_SIZE {
            for z in 0..TREE_SIZE {
                if ((x < (TREE_SIZE / 4) || y < (TREE_SIZE / 4) || z < (TREE_SIZE / 4))
                    && (0 == x % 2 && 0 == y % 4 && 0 == z % 2))
                    || ((TREE_SIZE / 2) <= x && (TREE_SIZE / 2) <= y && (TREE_SIZE / 2) <= z)
                {
                    tree.insert(
                        &V3c::new(x, y, z),
                        &Albedo::default()
                            .with_red((255 as f32 * x as f32 / TREE_SIZE as f32) as u8)
                            .with_green((255 as f32 * y as f32 / TREE_SIZE as f32) as u8)
                            .with_blue((255 as f32 * z as f32 / TREE_SIZE as f32) as u8)
                            .with_alpha(255),
                    )
                    .ok()
                    .unwrap();
                }
            }
        }
    }

    use shocovox_rs::octree::types::Albedo;
    use show_image::create_window;
    let window = create_window("image", Default::default()).ok().unwrap();

    let radius = 2. * TREE_SIZE as f32;
    let mut rng = rand::thread_rng();
    let mut angle = 40.;
    let mut velos = V3c::new(-0.05, 0., 0.);

    // Close app on window exit
    window
        .add_event_handler(|_, event, _| {
            match event {
                show_image::event::WindowEvent::Destroyed(_) => {
                    std::process::exit(0);
                }
                _ => {}
            };
        })
        .ok()
        .unwrap();

    loop {
        //generate a random number to add to velos
        velos = velos
            + V3c::new(
                (-5 + rng.gen_range(0..10)) as f32 / 2000.,
                (-5 + rng.gen_range(0..10)) as f32 / 2000.,
                (-5 + rng.gen_range(0..10)) as f32 / 2000.,
            );
        angle = angle + velos.x / 10.;

        // Set the viewport
        let origin = V3c::new(angle.sin() * radius, radius, angle.cos() * radius);
        let viewport_ray = Ray {
            direction: (V3c::unit(0.) - origin).normalized(),
            origin,
        };
        let viewport_up_direction = V3c::new(0., 1., 0.);
        let viewport_right_direction = viewport_up_direction
            .cross(viewport_ray.direction)
            .normalized();
        let viewport_width = 4.;
        let viewport_height = 4.;
        let viewport_fov = 3.;
        let pixel_width = viewport_width as f32 / viewport_size_width as f32;
        let pixel_height = viewport_height as f32 / viewport_size_height as f32;
        let viewport_bottom_left = viewport_ray.origin + (viewport_ray.direction * viewport_fov)
            - (viewport_up_direction * (viewport_height / 2.))
            - (viewport_right_direction * (viewport_width / 2.));

        // define light
        let diffuse_light_normal = V3c::new(0., -1., 1.).normalized();

        use image::ImageBuffer;
        use image::Rgb;
        let mut img = ImageBuffer::new(viewport_size_width, viewport_size_height);

        // cast each ray for a hit
        for x in 0..viewport_size_width {
            for y in 0..viewport_size_height {
                let actual_y_in_image = viewport_size_height - y - 1;
                //from the origin of the camera to the current point of the viewport
                let glass_point = viewport_bottom_left
                    + viewport_right_direction * x as f32 * pixel_width
                    + viewport_up_direction * y as f32 * pixel_height;
                let ray = Ray {
                    origin: viewport_ray.origin,
                    direction: (glass_point - viewport_ray.origin).normalized(),
                };

                use std::io::Write;
                std::io::stdout().flush().ok().unwrap();

                if let Some(hit) = tree.get_by_ray(&ray) {
                    let (data, _, normal) = hit;
                    //Because both vector should be normalized, the dot product should be 1*1*cos(angle)
                    //That means it is in range -1, +1, which should be accounted for
                    let diffuse_light_strength =
                        1. - (normal.dot(&diffuse_light_normal) / 2. + 0.5);
                    img.put_pixel(
                        x,
                        actual_y_in_image,
                        Rgb([
                            (data.albedo().unwrap().r as f32 * diffuse_light_strength) as u8,
                            (data.albedo().unwrap().g as f32 * diffuse_light_strength) as u8,
                            (data.albedo().unwrap().b as f32 * diffuse_light_strength) as u8,
                        ]),
                    );
                } else {
                    img.put_pixel(x, actual_y_in_image, Rgb([128, 128, 128]));
                }
            }
        }

        // img.save("example_junk_cpu_render.png").ok().unwrap();
        // std::process::exit(0);
        use show_image::{ImageInfo, ImageView};
        let binding = img.into_raw();
        let image = ImageView::new(
            ImageInfo::rgb8(viewport_size_width, viewport_size_height),
            &binding,
        );

        // Create a window with default options and display the image.
        window.set_image("image-001", image).ok().unwrap();
    }
}

#[cfg(not(feature = "raytracing"))]
fn main() {
    println!("You probably forgot to enable the raytracing feature!");
    //nothing to do when the feature is not enabled
}
