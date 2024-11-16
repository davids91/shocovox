#[cfg(feature = "bevy_wgpu")]
use bevy::{prelude::*, window::WindowPlugin};

#[cfg(feature = "bevy_wgpu")]
use shocovox_rs::octree::{
    raytracing::{OctreeGPUView, Ray, ShocoVoxRenderPlugin, Viewport},
    Albedo, Octree, V3c, VoxelData,
};

#[cfg(feature = "bevy_wgpu")]
use iyes_perf_ui::{
    entries::diagnostics::{PerfUiEntryFPS, PerfUiEntryFPSWorst},
    ui::root::PerfUiRoot,
    PerfUiPlugin,
};

#[cfg(feature = "bevy_wgpu")]
const DISPLAY_RESOLUTION: [u32; 2] = [1024, 768];

#[cfg(feature = "bevy_wgpu")]
const BRICK_DIMENSION: usize = 32;

#[cfg(feature = "bevy_wgpu")]
#[derive(Resource)]
struct TreeResource<T, const DIM: usize>
where
    T: Default + Clone + PartialEq + VoxelData,
{
    tree: Octree<T, DIM>,
}

#[cfg(feature = "bevy_wgpu")]
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    // uncomment for unthrottled FPS
                    present_mode: bevy::window::PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            ShocoVoxRenderPlugin {
                resolution: DISPLAY_RESOLUTION,
            },
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
    // fill octree with data
    let tree;
    let tree_path = "example_junk_minecraft";
    if std::path::Path::new(tree_path).exists() {
        tree = Octree::<Albedo, BRICK_DIMENSION>::load(&tree_path)
            .ok()
            .unwrap();
    } else {
        tree = match shocovox_rs::octree::Octree::<Albedo, BRICK_DIMENSION>::load_vox_file(
            "assets/models/minecraft.vox",
        ) {
            Ok(tree_) => tree_,
            Err(message) => panic!("Parsing model file failed with message: {message}"),
        };
        tree.save(&tree_path).ok().unwrap();
    }

    commands.spawn(DomePosition {
        yaw: 0.,
        roll: 0.,
        radius: tree.get_size() as f32 * 2.2,
    });

    let output_texture = tree.create_new_gpu_view(
        42,
        Viewport {
            origin: V3c {
                x: 0.,
                y: 0.,
                z: 0.,
            },
            direction: V3c {
                x: 0.,
                y: 0.,
                z: -1.,
            },
            w_h_fov: V3c::new(10., 10., 3.),
        },
        DISPLAY_RESOLUTION,
        &mut commands,
        images,
    );

    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(1024., 768.)),
            ..default()
        },
        texture: output_texture,
        ..default()
    });
    commands.spawn(Camera2dBundle::default());
    commands.insert_resource(TreeResource { tree });

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
fn rotate_camera(angles_query: Query<&mut DomePosition>, mut tree_view: ResMut<OctreeGPUView>) {
    let (yaw, roll) = (angles_query.single().yaw, angles_query.single().roll);
    let radius = angles_query.single().radius;

    tree_view.viewing_glass.viewport.origin = V3c::new(
        radius / 2. + yaw.sin() * radius,
        radius + roll.sin() * radius * 2.,
        radius / 2. + yaw.cos() * radius,
    );
    tree_view.viewing_glass.viewport.direction =
        (V3c::unit(radius / 2.) - tree_view.viewing_glass.viewport.origin).normalized();
}

#[cfg(feature = "bevy_wgpu")]
fn handle_zoom(
    keys: Res<ButtonInput<KeyCode>>,
    mut tree_view: ResMut<OctreeGPUView>,
    mut angles_query: Query<&mut DomePosition>,
    tree: Res<TreeResource<Albedo, BRICK_DIMENSION>>,
) {
    if keys.pressed(KeyCode::Delete) {
        tree_view.do_the_thing = true;
    }
    // if tree_view.is_changed() {
    //     println!("changed!! --> {:?}", tree_view.read_back);
    // }
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
            .cross(tree_view.viewing_glass.viewport.direction)
            .normalized();
        let pixel_width =
            tree_view.viewing_glass.viewport.w_h_fov.x as f32 / DISPLAY_RESOLUTION[0] as f32;
        let pixel_height =
            tree_view.viewing_glass.viewport.w_h_fov.y as f32 / DISPLAY_RESOLUTION[1] as f32;
        let viewport_bottom_left = tree_view.viewing_glass.viewport.origin
            + (tree_view.viewing_glass.viewport.direction
                * tree_view.viewing_glass.viewport.w_h_fov.z)
            - (viewport_up_direction * (tree_view.viewing_glass.viewport.w_h_fov.y / 2.))
            - (viewport_right_direction * (tree_view.viewing_glass.viewport.w_h_fov.x / 2.));

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
                    origin: tree_view.viewing_glass.viewport.origin,
                    direction: (glass_point - tree_view.viewing_glass.viewport.origin).normalized(),
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

    let multiplier = if keys.pressed(KeyCode::ShiftLeft) {
        10.0 // Doesn't have any effect?!
    } else {
        1.0
    };

    if keys.pressed(KeyCode::ArrowUp) {
        angles_query.single_mut().roll = angle_update_fn(angles_query.single().roll, ADDITION);
    }
    if keys.pressed(KeyCode::ArrowDown) {
        angles_query.single_mut().roll = angle_update_fn(angles_query.single().roll, -ADDITION);
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        angles_query.single_mut().yaw = angle_update_fn(angles_query.single().yaw, ADDITION);
        // println!("viewport: {:?}", viewing_glass.viewport);
    }
    if keys.pressed(KeyCode::ArrowRight) {
        angles_query.single_mut().yaw = angle_update_fn(angles_query.single().yaw, -ADDITION);
        // println!("viewport: {:?}", viewing_glass.viewport);
    }
    if keys.pressed(KeyCode::PageUp) {
        angles_query.single_mut().radius *= 1. - 0.02 * multiplier;
    }
    if keys.pressed(KeyCode::PageDown) {
        angles_query.single_mut().radius *= 1. + 0.02 * multiplier;
    }
    if keys.pressed(KeyCode::Home) {
        tree_view.viewing_glass.viewport.w_h_fov.x *= 1. + 0.09 * multiplier;
        tree_view.viewing_glass.viewport.w_h_fov.y *= 1. + 0.09 * multiplier;
    }
    if keys.pressed(KeyCode::End) {
        tree_view.viewing_glass.viewport.w_h_fov.x *= 1. - 0.09 * multiplier;
        tree_view.viewing_glass.viewport.w_h_fov.y *= 1. - 0.09 * multiplier;
    }
}

#[cfg(not(feature = "bevy_wgpu"))]
fn main() {
    println!("You probably forgot to enable the bevy_wgpu feature!");
    //nothing to do when the feature is not enabled
}
