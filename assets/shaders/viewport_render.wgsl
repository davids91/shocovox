// The time since startup data is in the globals binding which is part of the mesh_view_bindings import
#import bevy_pbr::mesh_view_bindings globals
#import bevy_pbr::mesh_vertex_output MeshVertexOutput

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

//crate::spatial::math::plane_line_intersection_distance
fn plane_line_intersection_distance(plane: Plane, line: Line) -> PlaneLineIntersection {
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
fn ray_at_point(ray: Line, d: f32) -> vec3f{
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
        let intersection = plane_line_intersection_distance(face, ray);
        if(intersection.hit){
            if(0. <= intersection.d && cube_contains_point(cube, ray_at_point(ray, intersection.d))){
                // ray hits the plane only when the resulting distance is at least positive,
                // and the point is contained inside the cube
                if(1 < distances_i && abs(distances[0] - distances[1]) < FLOAT_ERROR_TOLERANCE){
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
        result.exit_distance = distances[0];
    } else {
        result.hit = false;
        result.impact_hit = false;
    }
    return result;
}

struct NodeStackItem {
    bounds: Cube,
    node: u32,
    target_octant: u32,
    child_center: vec3f,
}

//crate::octree:raytracing::NodeStackItem::new
fn new_node_stack_item(bounds: Cube, node: u32, target_octant: u32) -> NodeStackItem {
    var result: NodeStackItem;
    result.bounds = bounds;
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

//crate::octree::raytracing::get_step_to_next_sibling
fn get_step_to_next_sibling(current: Cube, ray: Line) -> vec3f {
    let half_size = current.size / 2.;
    let midpoint = current.min_position + half_size;
    var sign_vec = sign(ray.direction);
    if(0. == sign_vec.x){ sign_vec.x = 1.; }
    if(0. == sign_vec.y){ sign_vec.y = 1.; }
    if(0. == sign_vec.z){ sign_vec.z = 1.; }
    var ref_point = midpoint + sign_vec * half_size;

    // Find the min of the 3 plane intersections
    let x_plane_distance = plane_line_intersection_distance(
        Plane(ref_point, vec3f(1., 0., 0.)), ray
    ).d;
    let y_plane_distance = plane_line_intersection_distance(
        Plane(ref_point, vec3f(0., 1., 0.)), ray
    ).d;
    let z_plane_distance = plane_line_intersection_distance(
        Plane(ref_point, vec3f(0., 0., 1.)), ray
    ).d;
    let min_d = min(x_plane_distance, min(y_plane_distance, z_plane_distance));

    // Step along the axes with the minimum distances
    var result = vec3f(0., 0., 0.);
    if( abs(min_d - x_plane_distance) < FLOAT_ERROR_TOLERANCE ) {
        result.x = sign_vec.x * current.size;
    }
    if( abs(min_d - y_plane_distance) < FLOAT_ERROR_TOLERANCE ) {
        result.y = sign_vec.y * current.size;
    }
    if( abs(min_d - z_plane_distance) < FLOAT_ERROR_TOLERANCE ) {
        result.z = sign_vec.z * current.size;
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
    var children_count = 8;
    if node.children[0] == key_none_value{
        children_count -= 1;
    }
    if node.children[1] == key_none_value{
        children_count -= 1;
    }
    if node.children[2] == key_none_value{
        children_count -= 1;
    }
    if node.children[3] == key_none_value{
        children_count -= 1;
    }
    if node.children[4] == key_none_value{
        children_count -= 1;
    }
    if node.children[5] == key_none_value{
        children_count -= 1;
    }
    if node.children[6] == key_none_value{
        children_count -= 1;
    }
    if node.children[7] == key_none_value{
        children_count -= 1;
    }

    return (node.contains_nodes == 1u && 0 == children_count);
}

struct OctreeRayIntersection {
    hit: bool,
    albedo : vec4<f32>,
    content: u32,
    collision_point: vec3f,
    impact_normal: vec3f,
}

const max_depth = 15; // the depth for an octree the size of 32768
fn get_by_ray(ray: Line) -> OctreeRayIntersection{
    var result: OctreeRayIntersection;

    var current_d: f32  = 0.0;
    var node_stack: array<NodeStackItem, max_depth>;
    var node_stack_i: i32 = 0;

    var root_bounds: Cube;
    root_bounds.min_position = vec3(0.,0.,0.);
    root_bounds.size = f32(octreeMetaData.root_size);
    let root_intersection = cube_intersect_ray(root_bounds, ray);
    if(root_intersection.hit){
        current_d = impact_or(root_intersection, 0.);
        if(is_leaf(nodes[OCTREE_ROOT_NODE_KEY])){
            result.hit = true;
            result.albedo = nodes[OCTREE_ROOT_NODE_KEY].albedo;
            result.content = nodes[OCTREE_ROOT_NODE_KEY].content;
            result.collision_point = ray_at_point(ray, current_d);
            result.impact_normal = root_intersection.impact_normal;
            return result;
        }
        let target_octant = hash_region(
            ray_at_point(ray, current_d) - root_bounds.min_position,
            f32(octreeMetaData.root_size),
        );
        node_stack[0] = new_node_stack_item(
            root_bounds, OCTREE_ROOT_NODE_KEY, target_octant
        );
        node_stack_i = 1;
    }

    var i = 0;
    while(0 < node_stack_i && node_stack_i < max_depth) { // until there are items on the stack

        // POP
        let current_bounds = node_stack[node_stack_i - 1].bounds;
        let current_bounds_ray_intersection = cube_intersect_ray(current_bounds, ray);
        if( (!cube_contains_point(current_bounds, node_stack[node_stack_i - 1].child_center))
            || (!current_bounds_ray_intersection.hit)
            || nodes[node_stack[node_stack_i - 1].node].contains_nodes == 0u
        ){
            let popped_target = node_stack[node_stack_i - 1];
            node_stack_i -= 1;
            if(0 < node_stack_i){
                let step_vec = get_step_to_next_sibling(popped_target.bounds, ray);
                node_stack[node_stack_i - 1] = add_point_to(node_stack[node_stack_i - 1], step_vec);
            }
            if(current_bounds_ray_intersection.hit){
                current_d = current_bounds_ray_intersection.exit_distance;
            }
            continue;
        }

        let current_node = node_stack[node_stack_i - 1].node;
        if(is_leaf(nodes[current_node]) && current_bounds_ray_intersection.hit){
            result.hit = true;
            result.albedo = nodes[current_node].albedo;
            result.content = nodes[current_node].content;
            result.collision_point = ray_at_point(ray, impact_or(current_bounds_ray_intersection, 0.));
            result.impact_normal = current_bounds_ray_intersection.impact_normal;
            return result;
        }

        if(current_bounds_ray_intersection.hit){
            current_d = impact_or(current_bounds_ray_intersection, current_d);
        }

        let current_target_octant = node_stack[node_stack_i - 1].target_octant;
        let target_child = nodes[current_node].children[current_target_octant];
        if(key_might_be_valid(target_child)) {
            // PUSH
            let child_bounds = child_bounds_for(current_bounds, current_target_octant);
            let child_target_octant = hash_region(
                (ray_at_point(ray, current_d) - child_bounds.min_position),
                child_bounds.size
            );
            node_stack[node_stack_i] = new_node_stack_item(
                child_bounds, target_child, child_target_octant
            );
            node_stack_i += 1;
        } else {
            // ADVANCE
            // target child is invalid, or it does not intersect with the ray
            // Advance iteration to the next sibling
            let dbg_octant = node_stack[node_stack_i - 1].target_octant;
            let current_target_bounds = target_bounds(node_stack[node_stack_i - 1]);
            let step_vec = get_step_to_next_sibling(current_target_bounds, ray);
            node_stack[node_stack_i - 1] = add_point_to(node_stack[node_stack_i - 1], step_vec);
        }
    }
    result.hit = false;
    return result;
}

struct SizedNode {
    contains_nodes: u32,
    albedo : vec4<f32>,
    content: u32,
    children: array<u32, 8>
}

const OCTREE_ROOT_NODE_KEY = 0u;
struct OctreeMetaData {
    root_size: u32
}

struct Viewport {
    origin: vec3f,
    direction: vec3f,
    size: vec2f,
    resolution: vec2f,
    fov: f32,
}

@group(1) @binding(0)
var<uniform> viewport: Viewport;

@group(1) @binding(1)
var<uniform> octreeMetaData: OctreeMetaData;

@group(1) @binding(2)
var<storage, read_write> nodes: array<SizedNode>;

@fragment
fn fragment(mesh: MeshVertexOutput) -> @location(0) vec4<f32> {
    let viewport_up_direction = vec3f(0., 1., 0.);
    let viewport_right_direction = normalize(cross(
        viewport_up_direction, viewport.direction
    ));
    let pixel_size = vec2f(
        viewport.size.x / viewport.resolution.x,
        viewport.size.y / viewport.resolution.y
    );
    let 
    viewport_bottom_left = viewport.origin 
        + (viewport.direction * viewport.fov)
        - (viewport_right_direction * (viewport.size.x / 2.))
        - (viewport_up_direction * (viewport.size.y / 2.))
        ;
    let glass_point = viewport_bottom_left
        + viewport_right_direction * viewport.size.x * (mesh.uv.x + pixel_size.x / 2.)
        + viewport_up_direction * viewport.size.y * ((1. - mesh.uv.y) + pixel_size.y / 2.)
        ;
    var ray = Line(glass_point, normalize(glass_point - viewport.origin));

    let ray_result = get_by_ray(ray);
    let diffuse_light_strength = (
        dot(ray_result.impact_normal, vec3f(-0.5,0.5,-0.5)) / 2. + 0.5
    );
    let result = ray_result.albedo.rgb * diffuse_light_strength;
    return vec4<f32>(result.rgb, 1.0);
}