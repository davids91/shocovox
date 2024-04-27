#[cfg(feature = "bevy_wgpu")]
use bevy::prelude::*;
#[cfg(feature = "bevy_wgpu")]
use shocovox_rs::{
    octree::raytracing::classic_raytracing_on_bevy_wgpu::OctreeViewMaterial, spatial::math::V3c,
};

#[cfg(feature = "bevy_wgpu")]
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<OctreeViewMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_camera)
        .add_systems(Update, handle_zoom)
        .run();
}

#[cfg(feature = "bevy_wgpu")]
const ARRAY_DIMENSION: u32 = 32;

#[cfg(feature = "bevy_wgpu")]
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OctreeViewMaterial>>,
) {
    use shocovox_rs::octree::raytracing::classic_raytracing_on_bevy_wgpu::Viewport;

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 3000.0,
            ..Default::default()
        },
        transform: Transform::from_xyz(-3.0, 2.0, -1.0),
        ..Default::default()
    });
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 3000.0,
            ..Default::default()
        },
        transform: Transform::from_xyz(3.0, 2.0, 1.0),
        ..Default::default()
    });

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(5.0, 5.0, 7.0).looking_at(Vec3::new(4., 1., 0.0), Vec3::Y),
        ..Default::default()
    });

    commands.spawn(DomePosition { yaw: 0. });

    // fill octree with data
    let mut tree = shocovox_rs::octree::Octree::<u32>::new(ARRAY_DIMENSION)
        .ok()
        .unwrap();

    tree.insert(&V3c::new(1, 3, 3), 0x66FFFF).ok().unwrap();
    for x in 0..ARRAY_DIMENSION {
        for y in 0..ARRAY_DIMENSION {
            for z in 0..ARRAY_DIMENSION {
                if x < (ARRAY_DIMENSION / 4)
                    || y < (ARRAY_DIMENSION / 4)
                    || z < (ARRAY_DIMENSION / 4)
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
                    tree.insert(&V3c::new(x, y, z), r | (g << 8) | (b << 16) | 0xFF000000)
                        .ok()
                        .unwrap();
                }
            }
        }
    }
    let quad_count = 2;
    let quad_size = 10. / quad_count as f32;
    let mesh_handle = meshes.add(Mesh::from(shape::Quad {
        size: Vec2::new(quad_size, quad_size),
        flip: false,
    }));
    let origin = Vec3::new(
        ARRAY_DIMENSION as f32 * 2.,
        ARRAY_DIMENSION as f32 / 2.,
        ARRAY_DIMENSION as f32 * -2.,
    );
    let material_handle = materials.add(tree.create_bevy_material_view(&Viewport {
        direction: (Vec3::new(0., 0., 0.) - origin).normalize(),
        origin,
        size: Vec2::new(10., 10.),
        fov: 3.,
    }));
    for x in 0..quad_count {
        commands.spawn(MaterialMeshBundle {
            mesh: mesh_handle.clone(),
            material: material_handle.clone(),
            transform: Transform::from_xyz((x as f32 * quad_size) + 0.5, x as f32 / 5., 0.0),
            ..Default::default()
        });
    }
}

#[cfg(feature = "bevy_wgpu")]
#[derive(Component)]
struct DomePosition {
    yaw: f32,
}

#[cfg(feature = "bevy_wgpu")]
fn rotate_camera(
    mut angles_query: Query<&mut DomePosition>,
    mut mats: ResMut<Assets<OctreeViewMaterial>>,
) {
    let angle = {
        let addition = ARRAY_DIMENSION as f32 / 1024.;
        let angle = angles_query.single().yaw + addition;
        if angle < 360. {
            angle
        } else {
            0.
        }
    };
    angles_query.single_mut().yaw = angle;

    for (_mat_handle, mat) in mats.as_mut().iter_mut() {
        let radius = ARRAY_DIMENSION as f32 * 1.3;
        mat.viewport.origin = Vec3::new(
            ARRAY_DIMENSION as f32 / 2. + angle.sin() * radius,
            ARRAY_DIMENSION as f32 / 2.,
            ARRAY_DIMENSION as f32 / 2. + angle.cos() * radius,
        );
        mat.viewport.direction = (Vec3::new(
            ARRAY_DIMENSION as f32 / 2.,
            ARRAY_DIMENSION as f32 / 2.,
            ARRAY_DIMENSION as f32 / 2.,
        ) - mat.viewport.origin)
            .normalize();
    }
}

#[cfg(feature = "bevy_wgpu")]
fn handle_zoom(keys: Res<ButtonInput<KeyCode>>, mut mats: ResMut<Assets<OctreeViewMaterial>>) {
    for (_mat_handle, mat) in mats.as_mut().iter_mut() {
        if keys.pressed(KeyCode::ArrowUp) {
            mat.viewport.size *= 1.1;
        }
        if keys.pressed(KeyCode::ArrowDown) {
            mat.viewport.size *= 0.9;
        }
    }
}

#[cfg(not(feature = "bevy_wgpu"))]
fn main() {} //nothing to do when the feature is not enabled
