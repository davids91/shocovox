#[cfg(feature = "bevy_wgpu")]
use bevy::{prelude::*, window::WindowPlugin};

#[cfg(feature = "bevy_wgpu")]
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

#[cfg(feature = "bevy_wgpu")]
use shocovox_rs::{
    octree::{BoxTree, V3c, V3cf32},
    raytracing::{OctreeGPUHost, Ray, SvxViewSet, Viewport},
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
const BRICK_DIMENSION: u32 = 32;

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
            shocovox_rs::raytracing::RenderBevyPlugin::<u32>::new(),
            bevy::diagnostic::FrameTimeDiagnosticsPlugin,
            PanOrbitCameraPlugin,
            PerfUiPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, set_viewport_for_camera)
        .add_systems(Update, handle_zoom)
        .run();
}

#[cfg(feature = "bevy_wgpu")]
fn setup(mut commands: Commands, images: ResMut<Assets<Image>>) {
    // fill octree with data

    use shocovox_rs::octree::MIPMapStrategy;
    let tree: BoxTree;
    let tree_path = "example_junk_beach";
    if std::path::Path::new(tree_path).exists() {
        tree = BoxTree::load(&tree_path).ok().unwrap();
    } else {
        tree = match MIPMapStrategy::default()
            .set_enabled(true)
            .load_vox_file(BRICK_DIMENSION, "assets/models/castle.vox")
        {
            Ok(tree_) => tree_,
            Err(message) => panic!("Parsing model file failed with message: {message}"),
        };
        tree.save(&tree_path).ok().unwrap();
    }

    let mut host = OctreeGPUHost { tree };
    let mut views = SvxViewSet::default();
    let view_index = host.create_new_view(
        &mut views,
        500,
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
            frustum: V3c::new(10., 10., 500.),
            fov: 3.,
        },
        DISPLAY_RESOLUTION,
        images,
    );

    commands.insert_resource(host);
    let mut display = Sprite::from_image(
        views.views[view_index]
            .lock()
            .unwrap()
            .output_texture()
            .clone(),
    );
    display.custom_size = Some(Vec2::new(1024., 768.));
    commands.spawn(display);
    commands.insert_resource(views);
    commands.spawn((
        Camera {
            is_active: false,
            ..default()
        },
        PanOrbitCamera {
            focus: Vec3::new(0., 300., 0.),
            ..default()
        },
    ));
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
fn direction_from_cam(cam: &PanOrbitCamera) -> Option<V3cf32> {
    if let Some(radius) = cam.radius {
        Some(
            V3c::new(
                radius / 2. + cam.yaw.unwrap().sin() * radius,
                radius + cam.pitch.unwrap().sin() * radius * 2.,
                radius / 2. + cam.yaw.unwrap().cos() * radius,
            )
            .normalized(),
        )
    } else {
        None
    }
}

#[cfg(feature = "bevy_wgpu")]
fn set_viewport_for_camera(camera_query: Query<&mut PanOrbitCamera>, view_set: ResMut<SvxViewSet>) {
    let cam = camera_query.single();
    if let Some(_) = cam.radius {
        let mut tree_view = view_set.views[0].lock().unwrap();
        tree_view.spyglass.viewport_mut().origin = V3c::new(cam.focus.x, cam.focus.y, cam.focus.z);
        tree_view.spyglass.viewport_mut().direction = direction_from_cam(cam).unwrap();
    }
}

#[cfg(feature = "bevy_wgpu")]
fn handle_zoom(
    keys: Res<ButtonInput<KeyCode>>,
    tree: ResMut<OctreeGPUHost>,
    view_set: ResMut<SvxViewSet>,
    mut camera_query: Query<&mut PanOrbitCamera>,
) {
    let mut tree_view = view_set.views[0].lock().unwrap();

    if keys.pressed(KeyCode::Tab) {
        // Render the current view with CPU
        let viewport_up_direction = V3c::new(0., 1., 0.);
        let viewport_right_direction = viewport_up_direction
            .cross(tree_view.spyglass.viewport().direction)
            .normalized();
        let pixel_width =
            tree_view.spyglass.viewport().frustum.x as f32 / DISPLAY_RESOLUTION[0] as f32;
        let pixel_height =
            tree_view.spyglass.viewport().frustum.y as f32 / DISPLAY_RESOLUTION[1] as f32;
        let viewport_bottom_left = tree_view.spyglass.viewport().origin
            + (tree_view.spyglass.viewport().direction * tree_view.spyglass.viewport().fov)
            - (viewport_up_direction * (tree_view.spyglass.viewport().frustum.y / 2.))
            - (viewport_right_direction * (tree_view.spyglass.viewport().frustum.x / 2.));

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
                    origin: tree_view.spyglass.viewport().origin,
                    direction: (glass_point - tree_view.spyglass.viewport().origin).normalized(),
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

        img.save("example_junk_cpu_render.png").ok().unwrap();
    }

    if keys.pressed(KeyCode::Home) {
        tree_view.spyglass.viewport_mut().fov *= 1. + 0.09;
    }
    if keys.pressed(KeyCode::End) {
        tree_view.spyglass.viewport_mut().fov *= 1. - 0.09;
    }

    let mut cam = camera_query.single_mut();
    if keys.pressed(KeyCode::ShiftLeft) {
        cam.target_focus.y += 1.;
    }
    if keys.pressed(KeyCode::ControlLeft) {
        cam.target_focus.y -= 1.;
    }
    if keys.pressed(KeyCode::NumpadAdd) {
        tree_view.spyglass.viewport_mut().frustum.z *= 1.01;
    }
    if keys.pressed(KeyCode::NumpadSubtract) {
        tree_view.spyglass.viewport_mut().frustum.z *= 0.99;
    }

    if keys.pressed(KeyCode::F3) {
        println!("{:?}", tree_view.spyglass.viewport());
    }

    if let Some(_) = cam.radius {
        let dir = direction_from_cam(&cam).unwrap();
        let dir = Vec3::new(dir.x, dir.y, dir.z);
        let right = dir.cross(Vec3::new(0., 1., 0.));
        if keys.pressed(KeyCode::KeyW) {
            cam.target_focus += dir;
        }
        if keys.pressed(KeyCode::KeyS) {
            cam.target_focus -= dir;
        }
        if keys.pressed(KeyCode::KeyA) {
            cam.target_focus += right;
        }
        if keys.pressed(KeyCode::KeyD) {
            cam.target_focus -= right;
        }
    }
}

#[cfg(not(feature = "bevy_wgpu"))]
fn main() {
    println!("You probably forgot to enable the bevy_wgpu feature!");
    //nothing to do when the feature is not enabled
}
