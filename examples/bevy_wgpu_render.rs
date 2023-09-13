#[cfg(feature = "bevy_wgpu")]
use bevy::prelude::*;
#[cfg(feature = "bevy_wgpu")]
use shocovox_rs::{octree::bevy_wgpu_octree::OctreeViewMaterial, spatial::math::V3c};

#[cfg(feature = "bevy_wgpu")]
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<OctreeViewMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_camera)
        .run();
}

#[cfg(feature = "bevy_wgpu")]
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OctreeViewMaterial>>,
) {
    use shocovox_rs::octree::bevy_wgpu_octree::Viewport;

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
        transform: Transform::from_xyz(5.0, 5.0, 10.0).looking_at(
            Vec3::new(2., 2., 0.0),
            Vec3::Y,
        ),
        ..Default::default()
    });

    commands.spawn(DomePosition { yaw: 0. });

    // fill octree with data
    let tree_size = 4;
    let mut tree = shocovox_rs::octree::Octree::<u32>::new(tree_size)
        .ok()
        .unwrap();

    tree.insert(&V3c::new(1, 3, 3), 0x66FFFF).ok();
    for x in 0..tree_size {
        for y in 0..tree_size {
            for z in 0..tree_size {
                if x < (tree_size / 4)
                    || y < (tree_size / 4)
                    || z < (tree_size / 4)
                    || ((tree_size / 2) <= x && (tree_size / 2) <= y && (tree_size / 2) <= z)
                {
                    tree.insert(&V3c::new(x, y, z), ((x + y + z) / (x * y * z).max(1)) * 10000).ok();
                }
            }
        }
    }
    let cube_count = 1;
    let cube_size = 10. / cube_count as f32;
    //    let mesh_handle = meshes.add(Mesh::from(shape::Cube { size: cube_size }));
    let mesh_handle = meshes.add(Mesh::from(shape::Quad {
        size: Vec2::new(cube_size, cube_size),
        flip: false,
    }));
    let material_handle = materials.add(tree.create_bevy_material_view(&Viewport {
        origin: Vec3::new(
            tree_size as f32 * 2.,
            tree_size as f32 / 2.,
            tree_size as f32 * -2.,
        ),
        direction: Vec3::new(-1., 0., 1.).normalize(),
        size: Vec2::new(tree_size as f32, tree_size as f32),
        resolution: Vec2::new(1024., 1024.),
        fov: 3.,
    }));
    for x in 0..cube_count {
        commands.spawn(MaterialMeshBundle {
            mesh: mesh_handle.clone(),
            material: material_handle.clone(),
            transform: Transform::from_xyz((x as f32 * cube_size) + 0.5, x as f32 / 5., 0.0),
            ..Default::default()
        });
    }
}

#[cfg(feature = "bevy_wgpu")]
#[derive(Component)]
struct DomePosition {
    yaw: f32,
}

fn rotate_camera(
    mut angles_query: Query<&mut DomePosition>,
    mut mats: ResMut<Assets<OctreeViewMaterial>>,
) {
    let angle = {
        let angle = angles_query.single().yaw + 0.005;
        if angle < 360. {
            angle
        } else {
            0.
        }
    };
    angles_query.single_mut().yaw = angle;

    for (_mat_handle, mat) in mats.as_mut().iter_mut() {
        let radius = 8.;
        mat.viewport.origin = Vec3::new(angle.sin() * radius, radius, angle.cos() * radius);
        mat.viewport.direction = (Vec3::new(5., 1., 0.) - mat.viewport.origin).normalize();
    }
}

#[cfg(not(feature = "bevy_wgpu"))]
fn main() {} //nothing to do when the feature is not enabled
