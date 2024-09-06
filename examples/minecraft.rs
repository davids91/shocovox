#[cfg(feature = "bevy_wgpu")]
use shocovox_rs::octree::Octree;

#[cfg(feature = "bevy_wgpu")]
use bevy::{prelude::*, window::WindowPlugin};

#[cfg(feature = "bevy_wgpu")]
use shocovox_rs::octree::{
    raytracing::{
        bevy::create_viewing_glass, ShocoVoxRenderPlugin, ShocoVoxViewingGlass, Viewport,
    },
    Albedo, V3c,
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
        tree = Octree::<Albedo, 16>::load(&tree_path).ok().unwrap();
    } else {
        tree = match shocovox_rs::octree::Octree::<Albedo, 16>::load_vox_file(
            "assets/models/minecraft.vox",
        ) {
            Ok(tree_) => tree_,
            Err(message) => panic!("Parsing model file failed with message: {message}"),
        };
        tree.save(&tree_path).ok().unwrap();
    }

    let origin = V3c::new(
        tree.get_size() as f32 * 2.,
        tree.get_size() as f32 / 2.,
        tree.get_size() as f32 * -2.,
    );
    commands.spawn(DomePosition {
        yaw: 0.,
        roll: 0.,
        radius: tree.get_size() as f32 * 2.2,
    });

    let render_data = tree.create_bevy_view();
    let viewing_glass = create_viewing_glass(
        &Viewport {
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
        images,
    );
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(1024., 768.)),
            ..default()
        },
        texture: viewing_glass.output_texture.clone(),
        ..default()
    });
    commands.spawn(Camera2dBundle::default());
    commands.insert_resource(render_data);
    commands.insert_resource(viewing_glass);

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
fn rotate_camera(
    angles_query: Query<&mut DomePosition>,
    mut viewing_glass: ResMut<ShocoVoxViewingGlass>,
) {
    let (yaw, roll) = (angles_query.single().yaw, angles_query.single().roll);

    let radius = angles_query.single().radius;
    viewing_glass.viewport.origin = V3c::new(
        radius / 2. + yaw.sin() * radius,
        radius + roll.sin() * radius * 2.,
        radius / 2. + yaw.cos() * radius,
    );
    viewing_glass.viewport.direction =
        (V3c::unit(radius / 2.) - viewing_glass.viewport.origin).normalized();
}

#[cfg(feature = "bevy_wgpu")]
fn handle_zoom(
    keys: Res<ButtonInput<KeyCode>>,
    mut viewing_glass: ResMut<ShocoVoxViewingGlass>,
    mut angles_query: Query<&mut DomePosition>,
) {
    const ADDITION: f32 = 0.05;
    let angle_update_fn = |angle, delta| -> f32 {
        let new_angle = angle + delta;
        if new_angle < 360. {
            new_angle
        } else {
            0.
        }
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
        angles_query.single_mut().radius *= 0.9;
    }
    if keys.pressed(KeyCode::PageDown) {
        angles_query.single_mut().radius *= 1.1;
    }
}

#[cfg(not(feature = "bevy_wgpu"))]
fn main() {
    println!("You probably forgot to enable the bevy_wgpu feature!");
    //nothing to do when the feature is not enabled
}
