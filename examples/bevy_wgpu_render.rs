#[cfg(feature = "bevy_wgpu")]
use bevy::{prelude::*, window::WindowPlugin};

#[cfg(feature = "bevy_wgpu")]
use shocovox_rs::octree::{
    raytracing::{ShocoVoxRenderPlugin, ShocoVoxViewingGlass, Viewport},
    Albedo, V3c,
};

#[cfg(feature = "bevy_wgpu")]
const DISPLAY_RESOLUTION: [u32; 2] = [1024, 768];

#[cfg(feature = "bevy_wgpu")]
const ARRAY_DIMENSION: u32 = 128;

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
    use shocovox_rs::octree::raytracing::bevy::create_viewing_glass;

    let origin = V3c::new(
        ARRAY_DIMENSION as f32 * 2.,
        ARRAY_DIMENSION as f32 / 2.,
        ARRAY_DIMENSION as f32 * -2.,
    );
    commands.spawn(DomePosition { yaw: 0. });

    // fill octree with data
    let mut tree = shocovox_rs::octree::Octree::<Albedo, 16>::new(ARRAY_DIMENSION)
        .ok()
        .unwrap();

    tree.insert(&V3c::new(1, 3, 3), Albedo::from(0x66FFFF))
        .ok()
        .unwrap();
    for x in 0..ARRAY_DIMENSION {
        for y in 0..ARRAY_DIMENSION {
            for z in 0..ARRAY_DIMENSION {
                if ((x < (ARRAY_DIMENSION / 4)
                    || y < (ARRAY_DIMENSION / 4)
                    || z < (ARRAY_DIMENSION / 4))
                    && (0 == x % 2 && 0 == y % 4 && 0 == z % 2))
                    || ((ARRAY_DIMENSION / 2) <= x
                        && (ARRAY_DIMENSION / 2) <= y
                        && (ARRAY_DIMENSION / 2) <= z)
                {
                    let r = if 0 == x % (ARRAY_DIMENSION / 4) {
                        (x as f32 / ARRAY_DIMENSION as f32 * 255.) as u32
                    } else {
                        128
                    };
                    let g = if 0 == y % (ARRAY_DIMENSION / 4) {
                        (y as f32 / ARRAY_DIMENSION as f32 * 255.) as u32
                    } else {
                        128
                    };
                    let b = if 0 == z % (ARRAY_DIMENSION / 4) {
                        (z as f32 / ARRAY_DIMENSION as f32 * 255.) as u32
                    } else {
                        128
                    };
                    tree.insert(
                        &V3c::new(x, y, z),
                        Albedo::default()
                            .with_red(r as u8)
                            .with_green(g as u8)
                            .with_blue(b as u8),
                    )
                    .ok()
                    .unwrap();
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
    mut angles_query: Query<&mut DomePosition>,
    mut viewing_glass: ResMut<ShocoVoxViewingGlass>,
) {
    let angle = {
        let addition = ARRAY_DIMENSION as f32 / 10000.;
        let angle = angles_query.single().yaw + addition;
        if angle < 360. {
            angle
        } else {
            0.
        }
    };
    angles_query.single_mut().yaw = angle;

    let radius = ARRAY_DIMENSION as f32 * 1.3;
    viewing_glass.viewport.origin = V3c::new(
        ARRAY_DIMENSION as f32 / 2. + angle.sin() * radius,
        ARRAY_DIMENSION as f32 + angle.cos() * angle.sin() * radius / 2.,
        ARRAY_DIMENSION as f32 / 2. + angle.cos() * radius,
    );
    viewing_glass.viewport.direction = (V3c::new(
        ARRAY_DIMENSION as f32 / 2.,
        ARRAY_DIMENSION as f32 / 2.,
        ARRAY_DIMENSION as f32 / 2.,
    ) - viewing_glass.viewport.origin)
        .normalized();
}

#[cfg(feature = "bevy_wgpu")]
fn handle_zoom(keys: Res<ButtonInput<KeyCode>>, mut viewing_glass: ResMut<ShocoVoxViewingGlass>) {
    if keys.pressed(KeyCode::ArrowUp) {
        //viewing_glass.viewport.w_h_fov *= 1.1;
        viewing_glass.viewport.w_h_fov.x *= 1.1;
        viewing_glass.viewport.w_h_fov.y *= 1.1;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        //viewing_glass.viewport.w_h_fov *= 0.9;
        viewing_glass.viewport.w_h_fov.x *= 0.9;
        viewing_glass.viewport.w_h_fov.y *= 0.9;
    }
}

#[cfg(not(feature = "bevy_wgpu"))]
fn main() {
    println!("You probably forgot to enable the bevy_wgpu feature!");
    //nothing to do when the feature is not enabled
}
