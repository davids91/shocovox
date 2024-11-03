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
    raytracing::{
        bevy::create_viewing_glass, ShocoVoxRenderPlugin, ShocoVoxViewingGlass, Viewport,
    },
    Albedo, V3c,
};

#[cfg(feature = "bevy_wgpu")]
const DISPLAY_RESOLUTION: [u32; 2] = [1024, 768];

#[cfg(feature = "bevy_wgpu")]
const BRICK_DIMENSION: usize = 2;

#[cfg(feature = "bevy_wgpu")]
const TREE_SIZE: u32 = 16;

#[cfg(feature = "bevy_wgpu")]
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin::default()),
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
    // use shocovox_rs::octree::raytracing::bevy::create_viewing_glass;

    let origin = V3c::new(
        TREE_SIZE as f32 * 2.,
        TREE_SIZE as f32 / 2.,
        TREE_SIZE as f32 * -2.,
    );

    // fill octree with data
    let mut tree = shocovox_rs::octree::Octree::<Albedo, BRICK_DIMENSION>::new(TREE_SIZE)
        .ok()
        .unwrap();

    // +++ DEBUG +++
    // tree.insert(&V3c::new(0, 0, 0), Albedo::from(0x66FFFF))
    //     .ok()
    //     .unwrap();
    // tree.insert(&V3c::new(3, 3, 3), Albedo::from(0x66FFFF))
    //     .ok()
    //     .unwrap();
    // assert!(tree.get(&V3c::new(3, 3, 3)).is_some());
    // tree.insert_at_lod(&V3c::new(0, 0, 0), 128, Albedo::from(0x66FFFF))
    //     .ok()
    //     .unwrap();

    // ---DEBUG ---
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

    let render_data = tree.create_bevy_view();
    let viewing_glass = create_viewing_glass(
        &Viewport {
            origin,
            direction: (V3c::new(0., 0., 0.) - origin).normalized(),
            w_h_fov: V3c::new(10., 10., 3.),
        },
        DISPLAY_RESOLUTION,
        images,
    );

    commands.spawn(DomePosition {
        yaw: 0.,
        roll: 0.,
        radius: tree.get_size() as f32 * 2.2,
    });
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
    const ADDITION: f32 = 0.02;
    let angle_update_fn = |angle, delta| -> f32 {
        let new_angle = angle + delta;
        if new_angle < 360. {
            new_angle
        } else {
            0.
        }
    };
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
        viewing_glass.viewport.w_h_fov.x *= 1. + 0.09 * multiplier;
        viewing_glass.viewport.w_h_fov.y *= 1. + 0.09 * multiplier;
    }
    if keys.pressed(KeyCode::End) {
        viewing_glass.viewport.w_h_fov.x *= 1. - 0.09 * multiplier;
        viewing_glass.viewport.w_h_fov.y *= 1. - 0.09 * multiplier;
    }
}

#[cfg(not(feature = "bevy_wgpu"))]
fn main() {
    println!("You probably forgot to enable the bevy_wgpu feature!");
    //nothing to do when the feature is not enabled
}
