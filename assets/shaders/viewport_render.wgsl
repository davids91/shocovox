// The time since startup data is in the globals binding which is part of the mesh_view_bindings import
#import bevy_pbr::{
    mesh_view_bindings::globals,
    forward_io::VertexOutput
}

struct Line {
    origin: vec3f,
    direction: vec3f,
}

struct Plane {
    point: vec3f,
    normal: vec3f,
}

struct Cube {
    min_position: vec3f,
    size: f32,
}

const FLOAT_ERROR_TOLERANCE = 0.00001;
//crate::spatial::raytracing::Cube::contains_point
fn cube_contains_point(cube: Cube, p: vec3f) -> bool{
    let min_cn = p >= cube.min_position - FLOAT_ERROR_TOLERANCE;
    let max_cn = p < (cube.min_position + cube.size + FLOAT_ERROR_TOLERANCE);
    return (
        min_cn.x && min_cn.y && min_cn.z && max_cn.x && max_cn.y && max_cn.z
    );
}

//Rust::unwrap_or
fn impact_or(impact: CubeRayIntersection, or: f32) -> f32{
    if(impact.hit && impact.impact_hit){
        return impact.impact_distance;
    }
    return or;
}

//crate::spatial::math::hash_region
fn hash_region(offset: vec3f, size: f32) -> u32 {
    let midpoint = vec3f(size / 2., size / 2., size / 2.);
    return u32(offset.x >= midpoint.x)
        + u32(offset.z >= midpoint.z) * 2u
        + u32(offset.y >= midpoint.y) * 4u;
}

//crate::spatial::math::offset_region
fn offset_region(octant: u32) -> vec3f {
    switch(octant){
        case 0u { return vec3f(0., 0., 0.); }
        case 1u { return vec3f(1., 0., 0.); }
        case 2u { return vec3f(0., 0., 1.); }
        case 3u { return vec3f(1., 0., 1.); }
        case 4u { return vec3f(0., 1., 0.); }
        case 5u { return vec3f(1., 1., 0.); }
        case 6u { return vec3f(0., 1., 1.); }
        case 7u, default { return vec3f(1.,1.,1.); }
    }
}

//crate::spatial::mod::Cube::child_bounds_for
fn child_bounds_for(bounds: Cube, octant: u32) -> Cube{
    var result: Cube;
    let child_size = bounds.size / 2.;
    result.min_position = bounds.min_position + (offset_region(octant) * child_size);
    result.size = child_size;
    return result;
}

struct PlaneLineIntersection {
    hit: bool,
    d: f32,
}

//crate::spatial::math::plane_line_intersection
fn plane_line_intersection(plane: Plane, line: Line) -> PlaneLineIntersection {
    var result: PlaneLineIntersection;
    let origins_diff = plane.point - line.origin;
    let plane_line_dot_to_plane = dot(origins_diff, plane.normal);
    let directions_dot = dot(line.direction, plane.normal);

    if 0. == directions_dot {
        // line and plane is paralell
        if 0. == dot(origins_diff, plane.normal) {
            // The distance is zero because the origin is already on the plane
            result.hit = true;
            result.d = 0.;
        } else {
            result.hit = false;
        }
    } else {
        result.hit = true;
        result.d = plane_line_dot_to_plane / directions_dot;
    }
    return result;
}

//crate::spatial::raytracing::Cube::face
fn get_cube_face(cube: Cube, face_index: u32) -> Plane{
    var result: Plane;
    switch(face_index){
        case 0u { result.normal = vec3f(0.,0.,-1.); }
        case 1u { result.normal = vec3f(-1.,0.,0.); }
        case 2u { result.normal = vec3f(0.,0.,1.); }
        case 3u { result.normal = vec3f(1.,0.,0.); }
        case 4u { result.normal = vec3f(0.,1.,0.); }
        case 5u, default { result.normal = vec3f(0.,-1.,0.); }
    }
    let half_size = cube.size / 2.;
    let midpoint = cube.min_position + half_size;
    result.point = midpoint + result.normal * half_size;
    return result;
}

struct CubeRayIntersection {
    hit: bool,
    impact_hit: bool,
    impact_distance: f32,
    exit_distance: f32,
    impact_normal: vec3f,
}

//crate::spatial::raytracing::Ray::point_at
fn point_in_ray_at_distance(ray: Line, d: f32) -> vec3f{
    return ray.origin + ray.direction * d;
}

//crate::spatial::raytracing::Cube::intersect_ray
fn cube_intersect_ray(cube: Cube, ray: Line) -> CubeRayIntersection{
    var result: CubeRayIntersection;
    var distances: array<f32, 2>; // An exit point and a potential impact point is needed to be stored
    var distances_i = 0;

    if cube_contains_point(cube, ray.origin) {
        distances[0] = 0.;
        distances_i = 1;
    }

    for(var cube_face_index: u32 = 0u; cube_face_index <= 6u; cube_face_index = cube_face_index + 1u){
        let face = get_cube_face(cube, cube_face_index);
        let intersection = plane_line_intersection(face, ray);
        if(intersection.hit){
            if(0. <= intersection.d && cube_contains_point(cube, point_in_ray_at_distance(ray, intersection.d))){
                // ray hits the plane only when the resulting distance is at least positive,
                // and the point is contained inside the cube
                if(
                    1 < distances_i
                    && (
                        abs(distances[0] - distances[1]) < FLOAT_ERROR_TOLERANCE
                        ||(
                            intersection.d < (distances[0] - FLOAT_ERROR_TOLERANCE)
                            && intersection.d < (distances[1] - FLOAT_ERROR_TOLERANCE)
                        )
                    )
                ){
                    // the first 2 hits were of an edge or the corner of the cube, so one of them can be discarded
                    distances[1] = intersection.d;
                } else if distances_i < 2 { // not enough hits are gathered yet
                    distances[distances_i] = intersection.d; 
                    distances_i = distances_i + 1;
                } else { // enough hits are gathered, exit the loop
                    break;
                }
                if 0 == distances_i || intersection.d <= distances[0] {
                    result.impact_normal = face.normal;
                }
            }
        }
    }
    if 1 < distances_i {
        result.hit = true;
        result.impact_hit = true;
        result.impact_distance = min(distances[0], distances[1]);
        result.exit_distance = max(distances[0], distances[1]);
    } else if 0 < distances_i {
        result.hit = true;
        result.impact_hit = false;
        result.impact_distance = 0.;
        result.exit_distance = distances[0];
    } else {
        result.hit = false;
        result.impact_hit = false;
        result.impact_distance = 0.;
        result.exit_distance = 0.;
    }
    return result;
}

struct NodeStackItem {
    bounds_intersection: CubeRayIntersection,
    bounds: Cube,
    node: u32,
    target_octant: u32,
    child_center: vec3f,
}

//crate::octree:raytracing::NodeStackItem::new
fn new_node_stack_item(bounds: Cube, cube_intersection: CubeRayIntersection, node: u32, target_octant: u32) -> NodeStackItem {
    var result: NodeStackItem;
    result.bounds = bounds;
    result.bounds_intersection = cube_intersection;
    result.node = node;
    result.target_octant = target_octant;
    result.child_center = (
        bounds.min_position + (bounds.size / 4.)
        + (offset_region(target_octant) * (result.bounds.size / 2.))
    );
    return result;
}

//crate::octree:raytracing::NodeStackItem::add_point
fn add_point_to(item: NodeStackItem, point: vec3f) -> NodeStackItem {
    var result: NodeStackItem = item;
    result.bounds = item.bounds;
    result.node = item.node;
    result.child_center = item.child_center + point;
    result.target_octant = hash_region(
        (result.child_center - result.bounds.min_position),
        result.bounds.size
    );
    return result;
}

//crate::octree:raytracing::NodeStackItem::target_bounds
//crate::spatial::Cube::child_bounds_for
fn target_bounds(item: NodeStackItem) -> Cube {
    var result: Cube;
    result.size = item.bounds.size / 2.;
    result.min_position = (
        item.bounds.min_position 
        + ( offset_region(item.target_octant) * result.size )
    );
    return result;
}

//crate::octree:raytracing::get_dda_scale_factors
fn get_dda_scale_factors(ray: Line) -> vec3f {
    var angle_corrected_direction = ray.direction;
    if 0. == ray.direction.x {
        angle_corrected_direction.x = FLOAT_ERROR_TOLERANCE;
    }
    if 0. == ray.direction.y {
        angle_corrected_direction.y = FLOAT_ERROR_TOLERANCE;
    }
    if 0. == ray.direction.z {
        angle_corrected_direction.z = FLOAT_ERROR_TOLERANCE;
    }
    return vec3f(
        sqrt(
            1.
            + pow(ray.direction.z / angle_corrected_direction.x, 2.)
            + pow(ray.direction.y / angle_corrected_direction.x, 2.)
        ),
        sqrt(
            pow(ray.direction.x / angle_corrected_direction.y, 2.)
            + 1.
            + pow(ray.direction.z / angle_corrected_direction.y, 2.)
        ),
        sqrt(
            pow(ray.direction.x / angle_corrected_direction.z, 2.)
            + pow(ray.direction.y / angle_corrected_direction.z, 2.)
            + 1.
        ),
    );
}

//crate::octree::raytracing::dda_step_to_next_sibling
fn dda_step_to_next_sibling(
    ray: Line, 
    ray_current_distance: ptr<function,f32>,
    current_bounds: Cube,
    ray_scale_factors: vec3f
) -> vec3f {
    var signum_vec = sign(ray.direction);
    let p = point_in_ray_at_distance(ray, *ray_current_distance);
    let steps_needed = (
        p - current_bounds.min_position
        - (current_bounds.size * max(sign(ray.direction), vec3f(0.,0.,0.)))
    );

    let d = (
        vec3f(*ray_current_distance, *ray_current_distance, *ray_current_distance) 
        + abs(steps_needed * ray_scale_factors)
    );
    *ray_current_distance = min(d.x, min(d.y, d.z));

    var result = vec3f(0., 0., 0.);
    if abs(*ray_current_distance - d.x) < FLOAT_ERROR_TOLERANCE {
        result.x = f32(abs(current_bounds.size)) * signum_vec.x;
    }
    if abs(*ray_current_distance - d.y) < FLOAT_ERROR_TOLERANCE {
        result.y = f32(abs(current_bounds.size)) * signum_vec.y;
    }
    if abs(*ray_current_distance - d.z) < FLOAT_ERROR_TOLERANCE {
        result.z = f32(abs(current_bounds.size)) * signum_vec.z;
    }
    return result;
}

const key_none_value : u32 = 4294967295u;

//crate::object_pool::key_might_be_valid
fn key_might_be_valid(key: u32) -> bool{
    return key < key_none_value;
}

//Unique to this implementation, not adapted from rust code
fn is_leaf(node: SizedNode) -> bool{
    if node.children[0] != key_none_value
    || node.children[1] != key_none_value
    || node.children[2] != key_none_value
    || node.children[3] != key_none_value
    || node.children[4] != key_none_value
    || node.children[5] != key_none_value
    || node.children[6] != key_none_value
    || node.children[7] != key_none_value {
        return false;
    }
    return node.contains_nodes <= (
        octreeMetaData.voxel_matrix_dim
        * octreeMetaData.voxel_matrix_dim
        * octreeMetaData.voxel_matrix_dim
    );
}

fn voxel_matrix_index_mapping(i: vec3u, dimensions: vec2u) -> u32 {
    return (i.x + (i.y * dimensions.y) + (i.z * dimensions.x * dimensions.y));
}

struct MatrixHit{
    hit: bool,
    index: vec3u
}

fn traverse_matrix(
    ray: Line,
    ray_current_distance: ptr<function,f32>,
    ray_scale_factors: vec3f,
    matrix_index_start: u32,
    bounds: Cube,
    intersection: CubeRayIntersection
) -> MatrixHit{
    let dimension = octreeMetaData.voxel_matrix_dim;
    var result: MatrixHit;
    result.hit = false;

    let pos = (
        point_in_ray_at_distance(
            ray, impact_or(intersection, *ray_current_distance)
        ) - bounds.min_position
    );
    var current_index = vec3i(
        clamp(i32(pos.x), 0, i32(dimension - 1)),
        clamp(i32(pos.y), 0, i32(dimension - 1)),
        clamp(i32(pos.z), 0, i32(dimension - 1))
    );
    let matrix_unit = bounds.size / f32(dimension);
    var current_bounds = Cube(
        bounds.min_position + vec3f(current_index) * matrix_unit,
        matrix_unit
    );
    loop{
        if current_index.x < 0
            || current_index.x >= i32(dimension)
            || current_index.y < 0
            || current_index.y >= i32(dimension)
            || current_index.z < 0
            || current_index.z >= i32(dimension)
        {
            result.hit = false;
            return result;
        }

        let voxel_matrix_index = u32(voxel_matrix_index_mapping(
            vec3u(current_index),
            vec2u(dimension, dimension)
        ));
        if !is_empty(voxels[matrix_index_start + voxel_matrix_index]) {
            result.hit = true;
            result.index = vec3u(current_index);
            return result;
        }

        let step = dda_step_to_next_sibling(
            ray,
            ray_current_distance,
            current_bounds,
            ray_scale_factors
        );
        current_bounds.min_position = current_bounds.min_position + vec3f(step);
        current_index = current_index + vec3i(step);
    }
    return result;
}

struct OctreeRayIntersection {
    hit: bool,
    albedo : vec4<f32>,
    content: u32,
    collision_point: vec3f,
    impact_normal: vec3f,
}

const max_depth = 20; // the depth for an octree the size of 1048576
                      // which would be approximately 10 km in case 1 voxel is 1 cm
fn get_by_ray(ray: Line) -> OctreeRayIntersection{
    var result: OctreeRayIntersection;

    var current_d: f32  = 0.0;
    var node_stack: array<NodeStackItem, max_depth>;
    var node_stack_i: i32 = 0;
    let dimension = octreeMetaData.voxel_matrix_dim;
    let ray_scale_factors = get_dda_scale_factors(ray);

    var root_bounds = Cube(vec3(0.,0.,0.), f32(octreeMetaData.octree_size));
    let root_intersection = cube_intersect_ray(root_bounds, ray);
    if(root_intersection.hit){
        current_d = impact_or(root_intersection, 0.);
        if is_leaf(nodes[OCTREE_ROOT_NODE_KEY]) {
            let root_matrix_hit = traverse_matrix(
                ray, &current_d, ray_scale_factors,
                nodes[OCTREE_ROOT_NODE_KEY].voxels_start_at,
                root_bounds, root_intersection
            );
            result.hit = root_matrix_hit.hit;
            if root_matrix_hit.hit == true {
                let hit_in_voxels = (
                    nodes[OCTREE_ROOT_NODE_KEY].voxels_start_at
                    + u32(voxel_matrix_index_mapping(
                        root_matrix_hit.index,
                        vec2u(dimension, dimension)
                    ))
                );
                let matrix_unit = root_bounds.size / f32(dimension);
                let result_bounds = Cube(
                    root_bounds.min_position + (
                        vec3f(root_matrix_hit.index) * matrix_unit
                    ),
                    matrix_unit
                );
                var result_raycast = cube_intersect_ray(result_bounds, ray);
                if result_raycast.hit == false {
                    result_raycast = root_intersection;
                }
                result.albedo = voxels[hit_in_voxels].albedo;
                result.content = voxels[hit_in_voxels].content;
                result.collision_point = point_in_ray_at_distance(
                    ray, impact_or(result_raycast, current_d)
                );
                result.impact_normal = result_raycast.impact_normal;
            }
            return result;
        }
        let target_octant = hash_region(
            point_in_ray_at_distance(ray, current_d) - root_bounds.min_position,
            root_bounds.size,
        );
        node_stack[0] = new_node_stack_item(
            root_bounds, root_intersection,
            OCTREE_ROOT_NODE_KEY, target_octant
        );
        node_stack_i = 1;
    }

    var i = 0;
    while(0 < node_stack_i && node_stack_i < max_depth) { // until there are items on the stack
        let current_bounds = node_stack[node_stack_i - 1].bounds;
        let current_bounds_ray_intersection = node_stack[node_stack_i - 1].bounds_intersection;
        if( (!cube_contains_point(current_bounds, node_stack[node_stack_i - 1].child_center))
            || nodes[node_stack[node_stack_i - 1].node].contains_nodes == 0u
        ){
            // POP
            let popped_target = node_stack[node_stack_i - 1];
            node_stack_i -= 1;
            if(0 < node_stack_i){
                let step_vec = dda_step_to_next_sibling(
                    ray,
                    &current_d,
                    popped_target.bounds,
                    ray_scale_factors
                );
                node_stack[node_stack_i - 1] = add_point_to(node_stack[node_stack_i - 1], step_vec);
            }
            current_d = current_bounds_ray_intersection.exit_distance;
            continue;
        }

        let current_node = node_stack[node_stack_i - 1].node;

        if is_leaf(nodes[current_node]) {
            let leaf_matrix_hit = traverse_matrix(
                ray, &current_d, ray_scale_factors,
                nodes[current_node].voxels_start_at,
                current_bounds, current_bounds_ray_intersection
            );
            if leaf_matrix_hit.hit == true {
                let hit_in_voxels = (
                    nodes[current_node].voxels_start_at
                    + u32(voxel_matrix_index_mapping(
                        leaf_matrix_hit.index,
                        vec2u(dimension, dimension)
                    ))
                );
                let matrix_unit = current_bounds.size / f32(dimension);
                let result_bounds = Cube(
                    current_bounds.min_position + (
                        vec3f(leaf_matrix_hit.index) * matrix_unit
                    ),
                    matrix_unit
                );
                var result_raycast = cube_intersect_ray(result_bounds, ray);
                if result_raycast.hit == false {
                    result_raycast = current_bounds_ray_intersection;
                }
                result.hit = true;
                result.albedo = voxels[hit_in_voxels].albedo;
                result.content = voxels[hit_in_voxels].content;
                result.collision_point = point_in_ray_at_distance(
                    ray, impact_or(result_raycast, current_d)
                );
                result.impact_normal = result_raycast.impact_normal;
                return result;
            } else {
                // POP
                let popped_target = node_stack[node_stack_i - 1];
                node_stack_i -= 1;
                if(0 < node_stack_i){
                    let step_vec = dda_step_to_next_sibling(
                        ray,
                        &current_d,
                        popped_target.bounds,
                        ray_scale_factors
                    );
                    node_stack[node_stack_i - 1] = add_point_to(node_stack[node_stack_i - 1], step_vec);
                }
                current_d = current_bounds_ray_intersection.exit_distance;
                continue;
            }
        }
        current_d = impact_or(current_bounds_ray_intersection, current_d);

        let target_octant = node_stack[node_stack_i - 1].target_octant;
        let target_child = nodes[current_node].children[target_octant];
        let target_bounds = child_bounds_for(current_bounds, target_octant);
        let target_is_empty = (
            !key_might_be_valid(target_child)
            || nodes[current_node].contains_nodes == 0u
        );

        let target_hit = cube_intersect_ray(target_bounds, ray);
        if(!target_is_empty && target_hit.hit) {
            // PUSH
            current_d = impact_or(target_hit, current_d);
            let child_target_octant = hash_region(
                (point_in_ray_at_distance(ray, current_d) - target_bounds.min_position),
                target_bounds.size
            );
            node_stack[node_stack_i] = new_node_stack_item(
                target_bounds, target_hit, target_child, child_target_octant
            );
            node_stack_i += 1;
        } else {
            // ADVANCE
            let current_target_bounds = target_bounds(node_stack[node_stack_i - 1]);
            let step_vec = dda_step_to_next_sibling(
                ray,
                &current_d,
                current_target_bounds,
                ray_scale_factors
            );
            node_stack[node_stack_i - 1] = add_point_to(node_stack[node_stack_i - 1], step_vec);
            if target_hit.hit == true {
                current_d = target_hit.exit_distance;
            }
        }
    }
    result.hit = false;
    return result;
}

struct Voxelement {
    albedo : vec4<f32>,
    content: u32,
}

fn is_empty(e: Voxelement) -> bool {
    return (
        0. == e.albedo.r
        && 0. == e.albedo.g
        && 0. == e.albedo.b
        && 0. == e.albedo.a
        && 0 == e.content
    );
}

struct SizedNode {
    contains_nodes: u32,
    children: array<u32, 8>,
    voxels_start_at: u32,
}

const OCTREE_ROOT_NODE_KEY = 0u;
struct OctreeMetaData {
    octree_size: u32,
    voxel_matrix_dim: u32,
    ambient_light_color: vec4f,
    ambient_light_position: vec3f,
}

struct Viewport {
    origin: vec3f,
    direction: vec3f,
    size: vec2f,
    fov: f32,
}

@group(2) @binding(0)
var<uniform> viewport: Viewport;

@group(2) @binding(1)
var<uniform> octreeMetaData: OctreeMetaData;

@group(2) @binding(2)
var<storage, read_write> nodes: array<SizedNode>;

@group(2) @binding(3)
var<storage, read_write> voxels: array<Voxelement>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let viewport_up_direction = vec3f(0., 1., 0.);
    let viewport_right_direction = normalize(cross(
        viewport_up_direction, viewport.direction
    ));
    let 
    viewport_bottom_left = viewport.origin 
        + (viewport.direction * viewport.fov)
        - (viewport_right_direction * (viewport.size.x / 2.))
        - (viewport_up_direction * (viewport.size.y / 2.))
        ;
    let ray_endpoint = viewport_bottom_left
        + viewport_right_direction * viewport.size.x * mesh.uv.x
        + viewport_up_direction * viewport.size.y * (1. - mesh.uv.y)
        ;
    var ray = Line(ray_endpoint, normalize(ray_endpoint - viewport.origin));

    var ray_result = get_by_ray(ray);
    var rgb_result = vec3f(0.5,0.5,0.5);
    if ray_result.hit == true {
        let diffuse_light_strength = (
            dot(ray_result.impact_normal, vec3f(-0.5,0.5,-0.5)) / 2. + 0.5
        );
        let result_with_lights = ray_result.albedo.rgb * diffuse_light_strength;
        rgb_result = result_with_lights.rgb;
    }
    return vec4<f32>(rgb_result, 1.0);
    //return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}