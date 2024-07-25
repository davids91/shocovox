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
const DISPLAY_RESOLUTION: [u32; 2] = [1024, 768];

#[cfg(feature = "bevy_wgpu")]
const TREE_SIZE: u32 = 64;

#[cfg(feature = "bevy_wgpu")]
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin::default()),
            ShocoVoxRenderPlugin {
                resolution: DISPLAY_RESOLUTION,
            },
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
    commands.spawn(DomePosition { yaw: 0. });

    // fill octree with data
    let tree;
    if std::path::Path::new("example_junk_minecraft_tree").exists() {
        tree = Octree::<Albedo, 16>::load("test_junk_octree").ok().unwrap();
    } else {
        tree = match shocovox_rs::octree::Octree::<Albedo, 16>::load_magica_voxel_file(
            "assets/models/minecraft.vox",
        ) {
            Ok(tree_) => tree_,
            Err(message) => panic!("Parsing model file failed with message: {message}"),
        };
        tree.save("example_junk_minecraft_tree");
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
}

#[cfg(feature = "bevy_wgpu")]
#[derive(Component)]
struct DomePosition {
    yaw: f32,
}

#[cfg(feature = "bevy_wgpu")]
fn rotate_camera(
    angles_query: Query<&mut DomePosition>,
    mut viewing_glass: ResMut<ShocoVoxViewingGlass>,
) {
    // let angle = {
    //     let addition = ARRAY_DIMENSION as f32 / 10.;
    //     let angle = angles_query.single().yaw + addition;
    //     if angle < 360. {
    //         angle
    //     } else {
    //         0.
    //     }
    // };
    // angles_query.single_mut().yaw = angle;

    let radius = TREE_SIZE as f32 * 2.5;
    let angle = angles_query.single().yaw;
    viewing_glass.viewport.origin = V3c::new(
        TREE_SIZE as f32 / 2. + angle.sin() * radius,
        TREE_SIZE as f32 + angle.cos() * angle.sin() * radius / 2.,
        TREE_SIZE as f32 / 2. + angle.cos() * radius,
    );
    viewing_glass.viewport.direction = (V3c::new(
        TREE_SIZE as f32 / 2.,
        TREE_SIZE as f32 / 2.,
        TREE_SIZE as f32 / 2.,
    ) - viewing_glass.viewport.origin)
        .normalized();
}

#[cfg(feature = "bevy_wgpu")]
fn handle_zoom(
    keys: Res<ButtonInput<KeyCode>>,
    mut viewing_glass: ResMut<ShocoVoxViewingGlass>,
    mut angles_query: Query<&mut DomePosition>,
) {
    if keys.pressed(KeyCode::ArrowUp) {
        viewing_glass.viewport.w_h_fov.x *= 1.1;
        viewing_glass.viewport.w_h_fov.y *= 1.1;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        viewing_glass.viewport.w_h_fov.x *= 0.9;
        viewing_glass.viewport.w_h_fov.y *= 0.9;
    }
    let addition = TREE_SIZE as f32 / 10.;
    if keys.pressed(KeyCode::ArrowLeft) {
        let angle = {
            let angle = angles_query.single().yaw - addition;
            if angle < 360. {
                angle
            } else {
                0.
            }
        };
        angles_query.single_mut().yaw = angle;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        let angle = {
            let angle = angles_query.single().yaw + addition;
            if angle < 360. {
                angle
            } else {
                0.
            }
        };
        angles_query.single_mut().yaw = angle;
    }
}

#[cfg(not(feature = "bevy_wgpu"))]
fn main() {
    println!("You probably forgot to enable the bevy_wgpu feature!");
    //nothing to do when the feature is not enabled
}
