#[cfg(feature = "bevy_wgpu")]
use bevy::{prelude::*, window::WindowPlugin};
#[cfg(feature = "bevy_wgpu")]
use iyes_perf_ui::{
    entries::diagnostics::{PerfUiEntryFPS, PerfUiEntryFPSWorst},
    ui::root::PerfUiRoot,
    PerfUiPlugin,
};

#[cfg(feature = "bevy_wgpu")]
use shocovox_rs::octree::{
    raytracing::{OctreeGPUHost, Ray, SvxViewSet, Viewport},
    Albedo, V3c,
};

#[cfg(feature = "bevy_wgpu")]
const DISPLAY_RESOLUTION: [u32; 2] = [1024, 768];

#[cfg(feature = "bevy_wgpu")]
const BRICK_DIMENSION: usize = 16;

#[cfg(feature = "bevy_wgpu")]
const TREE_SIZE: u32 = 128;

#[cfg(feature = "bevy_wgpu")]
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin::default()),
            shocovox_rs::octree::raytracing::RenderBevyPlugin::<Albedo, BRICK_DIMENSION>::new(
                DISPLAY_RESOLUTION,
            ),
            bevy::diagnostic::FrameTimeDiagnosticsPlugin,
            PerfUiPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_camera)
        .add_systems(Update, handle_zoom)
        .run();
}

#[cfg(feature = "bevy_wgpu")]
fn setup(mut commands: Commands, images: ResMut<Assets<Image>>) {
    let origin = V3c::new(
        TREE_SIZE as f32 * 2.,
        TREE_SIZE as f32 / 2.,
        TREE_SIZE as f32 * -2.,
    );

    // fill octree with data
    let mut tree = shocovox_rs::octree::Octree::<Albedo, BRICK_DIMENSION>::new(TREE_SIZE)
        .ok()
        .unwrap();

    for x in 0..TREE_SIZE {
        for y in 0..TREE_SIZE {
            for z in 0..TREE_SIZE {
                if ((x < (TREE_SIZE / 4) || y < (TREE_SIZE / 4) || z < (TREE_SIZE / 4))
                    && (0 == x % 2 && 0 == y % 4 && 0 == z % 2))
                    || ((TREE_SIZE / 2) <= x && (TREE_SIZE / 2) <= y && (TREE_SIZE / 2) <= z)
                {
                    let r = if 0 == x % (TREE_SIZE / 4) {
                        (x as f32 / TREE_SIZE as f32 * 255.) as u32
                    } else {
                        128
                    };
                    let g = if 0 == y % (TREE_SIZE / 4) {
                        (y as f32 / TREE_SIZE as f32 * 255.) as u32
                    } else {
                        128
                    };
                    let b = if 0 == z % (TREE_SIZE / 4) {
                        (z as f32 / TREE_SIZE as f32 * 255.) as u32
                    } else {
                        128
                    };
                    // println!("Inserting at: {:?}", (x, y, z));
                    tree.insert(
                        &V3c::new(x, y, z),
                        Albedo::default()
                            .with_red(r as u8)
                            .with_green(g as u8)
                            .with_blue(b as u8)
                            .with_alpha(255),
                    )
                    .ok()
                    .unwrap();
                    assert!(tree.get(&V3c::new(x, y, z)).is_some());
                }
            }
        }
    }

    commands.spawn(DomePosition {
        yaw: 0.,
        roll: 0.,
        radius: tree.get_size() as f32 * 2.2,
    });

    let mut host = OctreeGPUHost { tree };
    let mut views = SvxViewSet::default();
    let output_texture = host.create_new_view(
        &mut views,
        45,
        Viewport {
            origin,
            direction: (V3c::new(0., 0., 0.) - origin).normalized(),
            w_h_fov: V3c::new(10., 10., 3.),
        },
        DISPLAY_RESOLUTION,
        images,
    );
    commands.insert_resource(host);
    commands.insert_resource(views);
    commands.spawn(Sprite::from_image(output_texture));
    commands.spawn(Camera2d::default());
    commands.spawn((
        PerfUiRoot::default(),
        PerfUiEntryFPS {
            label: "Frame Rate (current)".into(),
            threshold_highlight: Some(60.0),
            digits: 5,
            precision: 2,
            ..default()
        },
        PerfUiEntryFPSWorst {
            label: "Frame Rate (worst)".into(),
            threshold_highlight: Some(60.0),
            digits: 5,
            precision: 2,
            ..default()
        },
    ));
}

#[cfg(feature = "bevy_wgpu")]
#[derive(Component)]
struct DomePosition {
    radius: f32,
    yaw: f32,
    roll: f32,
}

#[cfg(feature = "bevy_wgpu")]
fn rotate_camera(angles_query: Query<&mut DomePosition>, view_set: ResMut<SvxViewSet>) {
    let (yaw, roll) = (angles_query.single().yaw, angles_query.single().roll);
    let radius = angles_query.single().radius;
    let mut tree_view = view_set.views[0].lock().unwrap();
    tree_view.spyglass.viewport.origin = V3c::new(
        radius / 2. + yaw.sin() * radius,
        radius + roll.sin() * radius * 2.,
        radius / 2. + yaw.cos() * radius,
    );
    tree_view.spyglass.viewport.direction =
        (V3c::unit(radius / 2.) - tree_view.spyglass.viewport.origin).normalized();
}

#[cfg(feature = "bevy_wgpu")]
fn handle_zoom(
    keys: Res<ButtonInput<KeyCode>>,
    tree: ResMut<OctreeGPUHost<Albedo, BRICK_DIMENSION>>,
    view_set: ResMut<SvxViewSet>,
    mut angles_query: Query<&mut DomePosition>,
) {
    let mut tree_view = view_set.views[0].lock().unwrap();
    const ADDITION: f32 = 0.05;
    let angle_update_fn = |angle, delta| -> f32 {
        let new_angle = angle + delta;
        if new_angle < 360. {
            new_angle
        } else {
            0.
        }
    };
    if keys.pressed(KeyCode::Tab) {
        // Render the current view with CPU
        let viewport_up_direction = V3c::new(0., 1., 0.);
        let viewport_right_direction = viewport_up_direction
            .cross(tree_view.spyglass.viewport.direction)
            .normalized();
        let pixel_width =
            tree_view.spyglass.viewport.w_h_fov.x as f32 / DISPLAY_RESOLUTION[0] as f32;
        let pixel_height =
            tree_view.spyglass.viewport.w_h_fov.y as f32 / DISPLAY_RESOLUTION[1] as f32;
        let viewport_bottom_left = tree_view.spyglass.viewport.origin
            + (tree_view.spyglass.viewport.direction * tree_view.spyglass.viewport.w_h_fov.z)
            - (viewport_up_direction * (tree_view.spyglass.viewport.w_h_fov.y / 2.))
            - (viewport_right_direction * (tree_view.spyglass.viewport.w_h_fov.x / 2.));

        // define light
        let diffuse_light_normal = V3c::new(0., -1., 1.).normalized();

        use image::ImageBuffer;
        use image::Rgb;
        let mut img = ImageBuffer::new(DISPLAY_RESOLUTION[0], DISPLAY_RESOLUTION[1]);

        // cast each ray for a hit
        for x in 0..DISPLAY_RESOLUTION[0] {
            for y in 0..DISPLAY_RESOLUTION[1] {
                let actual_y_in_image = DISPLAY_RESOLUTION[1] - y - 1;
                //from the origin of the camera to the current point of the viewport
                let glass_point = viewport_bottom_left
                    + viewport_right_direction * x as f32 * pixel_width
                    + viewport_up_direction * y as f32 * pixel_height;
                let ray = Ray {
                    origin: tree_view.spyglass.viewport.origin,
                    direction: (glass_point - tree_view.spyglass.viewport.origin).normalized(),
                };

                use std::io::Write;
                std::io::stdout().flush().ok().unwrap();

                if let Some(hit) = tree.tree.get_by_ray(&ray) {
                    let (data, _, normal) = hit;
                    //Because both vector should be normalized, the dot product should be 1*1*cos(angle)
                    //That means it is in range -1, +1, which should be accounted for
                    let diffuse_light_strength =
                        1. - (normal.dot(&diffuse_light_normal) / 2. + 0.5);
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
                    img.put_pixel(x, actual_y_in_image, Rgb([128, 128, 128]));
                }
            }
        }

        img.save("example_junk_cpu_render.png").ok().unwrap();
    }

    if keys.pressed(KeyCode::ArrowUp) {
        angles_query.single_mut().roll = angle_update_fn(angles_query.single().roll, ADDITION);
    }
    if keys.pressed(KeyCode::ArrowDown) {
        angles_query.single_mut().roll = angle_update_fn(angles_query.single().roll, -ADDITION);
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        angles_query.single_mut().yaw = angle_update_fn(angles_query.single().yaw, ADDITION);
    }
    if keys.pressed(KeyCode::ArrowRight) {
        angles_query.single_mut().yaw = angle_update_fn(angles_query.single().yaw, -ADDITION);
    }
    if keys.pressed(KeyCode::PageUp) {
        angles_query.single_mut().radius *= 1. - 0.02;
    }
    if keys.pressed(KeyCode::PageDown) {
        angles_query.single_mut().radius *= 1. + 0.02;
    }
    if keys.pressed(KeyCode::Home) {
        tree_view.spyglass.viewport.w_h_fov.z *= 1. + 0.09;
    }
    if keys.pressed(KeyCode::End) {
        tree_view.spyglass.viewport.w_h_fov.z *= 1. - 0.09;
    }
}

#[cfg(not(feature = "bevy_wgpu"))]
fn main() {
    println!("You probably forgot to enable the bevy_wgpu feature!");
    //nothing to do when the feature is not enabled
}
