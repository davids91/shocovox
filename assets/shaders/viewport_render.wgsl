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
const OOB_OCTANT = 8u;

//crate::spatial::raytracing::Cube::contains_point
fn cube_contains_point(cube: Cube, p: vec3f) -> bool{
    return (
        (p.x >= cube.min_position.x - FLOAT_ERROR_TOLERANCE)
        &&(p.y >= cube.min_position.y - FLOAT_ERROR_TOLERANCE)
        &&(p.z >= cube.min_position.z - FLOAT_ERROR_TOLERANCE)
        &&(p.x < (cube.min_position.x + cube.size + FLOAT_ERROR_TOLERANCE))
        &&(p.y < (cube.min_position.y + cube.size + FLOAT_ERROR_TOLERANCE))
        &&(p.z < (cube.min_position.z + cube.size + FLOAT_ERROR_TOLERANCE))
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
fn hash_region(offset: vec3f, size_half: f32) -> u32 {
    return u32(offset.x >= size_half)
        + u32(offset.z >= size_half) * 2u
        + u32(offset.y >= size_half) * 4u;
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
    return Cube(
        bounds.min_position + (offset_region(octant) * bounds.size / 2.),
        round(bounds.size / 2.)
    );
}

struct PlaneLineIntersection {
    hit: bool,
    d: f32,
}


//crate::spatial::math::plane_line_intersection
fn plane_line_intersection(plane: Plane, line: Line) -> PlaneLineIntersection {
    let directions_dot = dot(line.direction, plane.normal);
    if 0. == directions_dot {
        // line and plane is paralell
        if 0. == dot(plane.point - line.origin, plane.normal) {
            // The distance is zero because the origin is already on the plane
            return PlaneLineIntersection(true, 0.);
        } else {
            return PlaneLineIntersection(false, 0.);
        }
    } else {
        return PlaneLineIntersection(
            true,
            dot(plane.point - line.origin, plane.normal) / directions_dot
        );
    }
}

//crate::spatial::raytracing::Cube::face
fn get_cube_face(cube: Cube, face_index: u32) -> Plane{
    var result_normal: vec3f;
    switch(face_index){
        case 0u { result_normal = vec3f(0.,0.,-1.); }
        case 1u { result_normal = vec3f(-1.,0.,0.); }
        case 2u { result_normal = vec3f(0.,0.,1.); }
        case 3u { result_normal = vec3f(1.,0.,0.); }
        case 4u { result_normal = vec3f(0.,1.,0.); }
        case 5u, default { result_normal = vec3f(0.,-1.,0.); }
    }
    return Plane(
        cube.min_position + cube.size / 2. + result_normal * cube.size / 2.,
        result_normal
    );
}

struct CubeRayIntersection {
    hit: bool,
    impact_hit: bool,
    impact_distance: f32,
    exit_distance: f32,
}

//crate::spatial::raytracing::Ray::point_at
fn point_in_ray_at_distance(ray: Line, d: f32) -> vec3f{
    return ray.origin + ray.direction * d;
}

//crate::spatial::raytracing::Cube::intersect_ray
fn cube_intersect_ray(cube: Cube, ray: Line) -> CubeRayIntersection{
    let max_position = cube.min_position + vec3f(cube.size, cube.size, cube.size);
    let tmin = max(
        max(
            min(
                (cube.min_position.x - ray.origin.x) / ray.direction.x,
                (max_position.x - ray.origin.x) / ray.direction.x
            ),
            min(
                (cube.min_position.y - ray.origin.y) / ray.direction.y,
                (max_position.y - ray.origin.y) / ray.direction.y
            )
        ),
        min(
            (cube.min_position.z - ray.origin.z) / ray.direction.z,
            (max_position.z - ray.origin.z) / ray.direction.z
        )
    );
    let tmax = min(
        min(
            max(
                (cube.min_position.x - ray.origin.x) / ray.direction.x,
                (max_position.x - ray.origin.x) / ray.direction.x
            ),
            max(
                (cube.min_position.y - ray.origin.y) / ray.direction.y,
                (max_position.y - ray.origin.y) / ray.direction.y
            )
        ),
        max(
            (cube.min_position.z - ray.origin.z) / ray.direction.z,
            (max_position.z - ray.origin.z) / ray.direction.z
        )
    );

    if tmax < 0. || tmin > tmax{
        return CubeRayIntersection(false, false, 0., 0.);
    }

    if tmin < 0.0 {
        return CubeRayIntersection(true, false, 0., tmax);
    }

    return CubeRayIntersection(true, true, tmin, tmax);
}

fn cube_impact_normal(cube: Cube, impact_point: vec3f) -> vec3f{
    var impact_normal = vec3f(0.,0.,0.);
    let mid_to_impact = cube.min_position + vec3f(cube.size / 2.) - impact_point;
    let max_component = max(
        abs(mid_to_impact).x,
        max(abs(mid_to_impact).y, abs(mid_to_impact).z)
    );
    if max_component - abs(mid_to_impact).x < FLOAT_ERROR_TOLERANCE {
        impact_normal.x = -mid_to_impact.x;
    }
    if max_component - abs(mid_to_impact).y < FLOAT_ERROR_TOLERANCE {
        impact_normal.y = -mid_to_impact.y;
    }
    if max_component - abs(mid_to_impact).z < FLOAT_ERROR_TOLERANCE {
        impact_normal.z = -mid_to_impact.z;
    }
    return normalize(impact_normal);
}


//crate::octree::raytracing::NodeStack
const NODE_STACK_SIZE: u32 = 4;
const EMPTY_MARKER: u32 = 0xFFFFFFFFu;

//crate::octree::raytracing::NodeStack::is_empty
fn node_stack_is_empty(node_stack_meta: u32) -> bool {
    return 0 == (node_stack_meta & 0x000000FFu);
}

//crate::octree::raytracing::NodeStack::push
fn node_stack_push(
    node_stack: ptr<function,array<u32, NODE_STACK_SIZE>>,
    node_stack_meta: ptr<function, u32>,
    data: u32,
){
    *node_stack_meta = (
        // count
        ( min(NODE_STACK_SIZE, ((*node_stack_meta & 0x000000FFu) + 1)) & 0x000000FFu)
        // head_index
        | ( ((
            ( ((*node_stack_meta & 0x0000FF00u) >> 8u) + 1 ) % NODE_STACK_SIZE
        ) << 8u) & 0x0000FF00u )
    );
    (*node_stack)[(*node_stack_meta & 0x0000FF00u) >> 8u] = data;
}


//crate::octree::raytracing::NodeStack::pop
fn node_stack_pop(
    node_stack: ptr<function,array<u32, NODE_STACK_SIZE>>,
    node_stack_meta: ptr<function, u32>,
) -> u32 { // returns either with index or EMPTY_MARKER
    if 0 == (*node_stack_meta & 0x000000FFu) {
        return EMPTY_MARKER;
    }
    let result = (*node_stack)[(*node_stack_meta & 0x0000FF00u) >> 8u];
    if 0 == (*node_stack_meta & 0x0000FF00u) { // head index is 0
        *node_stack_meta = (
            // count
            ( ((*node_stack_meta & 0x000000FFu) - 1) )
            // head_index
            | ((NODE_STACK_SIZE - 1) << 8u)
        );
    } else {
        *node_stack_meta = (
            // count
            ( ((*node_stack_meta & 0x000000FFu) - 1) )
            // head_index
            | ( ((
                ( ((*node_stack_meta & 0x0000FF00u) >> 8u) - 1 )
            ) << 8u) & 0x0000FF00u )
        );
    }
    return result;
}

//crate::octree::raytracing::NodeStack::last/last_mut
fn node_stack_last(node_stack_meta: u32) -> u32 { // returns either with index or EMPTY_MARKER
    if 0 == (node_stack_meta & 0x000000FFu) {
        return EMPTY_MARKER;
    }
    return (node_stack_meta & 0x0000FF00u) >> 8u;
}

//crate::octree:raytracing::get_dda_scale_factors
fn get_dda_scale_factors(ray: Line) -> vec3f {
    return vec3f(
        sqrt(
            1.
            + pow(ray.direction.z / ray.direction.x, 2.)
            + pow(ray.direction.y / ray.direction.x, 2.)
        ),
        sqrt(
            pow(ray.direction.x / ray.direction.y, 2.)
            + 1.
            + pow(ray.direction.z / ray.direction.y, 2.)
        ),
        sqrt(
            pow(ray.direction.x / ray.direction.z, 2.)
            + pow(ray.direction.y / ray.direction.z, 2.)
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
    let d = (
        vec3f(*ray_current_distance, *ray_current_distance, *ray_current_distance) 
        + abs(
            ( // steps needed to reach next axis
                (current_bounds.size * max(sign(ray.direction), vec3f(0.,0.,0.)))
                - (
                    sign(ray.direction) 
                    * (
                        point_in_ray_at_distance(ray, *ray_current_distance)
                        - current_bounds.min_position
                    )
                )
            )
            * ray_scale_factors
        )
    );
    *ray_current_distance = min(d.x, min(d.y, d.z));

    var result = vec3f(0., 0., 0.);
    if abs(*ray_current_distance - d.x) < FLOAT_ERROR_TOLERANCE {
        result.x = sign(ray.direction).x;
    }
    if abs(*ray_current_distance - d.y) < FLOAT_ERROR_TOLERANCE {
        result.y = sign(ray.direction).y;
    }
    if abs(*ray_current_distance - d.z) < FLOAT_ERROR_TOLERANCE {
        result.z = sign(ray.direction).z;
    }
    return result;
}

// Unique to this implementation, not adapted from rust code, corresponds to:
//crate::octree::raytracing::classic_raytracing_on_bevy_wgpu::meta_set_is_leaf
fn is_leaf(sized_node_meta: u32) -> bool {
    return 0 < (0x01000000 & sized_node_meta);
}

// Unique to this implementation, not adapted from rust code, corresponds to:
//crate::octree::raytracing::classic_raytracing_on_bevy_wgpu::meta_set_node_occupancy_bitmap
fn get_node_occupancy_bitmap(sized_node_meta: u32) -> u32 {
    return (0x000000FF & sized_node_meta);
}

//crate::spatial::math::step_octant
fn step_octant(octant: u32, step: vec3f) -> u32 {
    return (
        (
            OCTANT_STEP_RESULT_LUT[u32(sign(step.x) + 1)][u32(sign(step.y) + 1)][u32(sign(step.z) + 1)]
            & (0x0Fu << (4 * octant))
        ) >> (4 * octant)
    ) & 0x0Fu;
}

//crate::spatial::math::hash_direction
fn hash_direction(direction: vec3f) -> u32 {
    return hash_region(vec3f(1.) + normalize(direction), 1.);
}

// Functionality-wise this function is more generic, than its counterpart
// and is used in voxel brick mapping too
//crate::spatial::math::flat_projection
fn flat_projection(i: vec3u, dimensions: vec2u) -> u32 {
    return (i.x + (i.y * dimensions.y) + (i.z * dimensions.x * dimensions.y));
}

//crate::spatial::math::position_in_bitmap_64bits
fn position_in_bitmap_64bits(i: vec3u, dimension: u32) -> u32{
    return flat_projection(
        i * 4 / dimension, vec2u(4, 4)
    );
}

// Unique to this implementation, not adapted from rust code
fn get_occupancy_in_bitmap_64bits(
    bit_position: u32,
    bitmap_lsb: u32,
    bitmap_msb: u32
) -> bool {
    // not possible to create a position mask directly, because of missing u64 type
    if bit_position < 32 {
        return 0 < (bitmap_lsb & u32(0x01u << bit_position));
    }
    return 0 < (bitmap_msb & u32(0x01u << (bit_position - 32)));
}

struct BrickHit{
    hit: bool,
    index: vec3u
}

fn traverse_brick(
    ray: Line,
    ray_current_distance: ptr<function,f32>,
    brick_index_start: u32,
    occupancy_bitmap_lsb: u32,
    occupancy_bitmap_msb: u32,
    brick_bounds: Cube,
    ray_scale_factors: vec3f,
    direction_lut_index: u32,
) -> BrickHit{
    var current_index = vec3i(
        point_in_ray_at_distance(ray, *ray_current_distance)
        - brick_bounds.min_position
    );
    current_index = vec3i(
        clamp(i32(current_index.x), 0, i32(octreeMetaData.voxel_brick_dim - 1)),
        clamp(i32(current_index.y), 0, i32(octreeMetaData.voxel_brick_dim - 1)),
        clamp(i32(current_index.z), 0, i32(octreeMetaData.voxel_brick_dim - 1))
    );

    var current_bounds = Cube(
        (
            brick_bounds.min_position 
            + vec3f(current_index) * (brick_bounds.size / f32(octreeMetaData.voxel_brick_dim))
        ),
        (brick_bounds.size / f32(octreeMetaData.voxel_brick_dim))
    );

    var mapped_index = position_in_bitmap_64bits(
        vec3u(current_index), octreeMetaData.voxel_brick_dim
    );
    if (
        0 == (
            RAY_TO_LEAF_OCCUPANCY_BITMASK_LUT[mapped_index][direction_lut_index * 2]
            & occupancy_bitmap_lsb
        )
        && 0 == (
            RAY_TO_LEAF_OCCUPANCY_BITMASK_LUT[mapped_index][direction_lut_index * 2 + 1]
            & occupancy_bitmap_msb
        )
    ){
        return BrickHit(false, vec3u());
    }

    var prev_bitmap_position_full_resolution = vec3u(
        vec3f(current_index) * (4. / f32(octreeMetaData.voxel_brick_dim))
    );
    loop{
        if current_index.x < 0
            || current_index.x >= i32(octreeMetaData.voxel_brick_dim)
            || current_index.y < 0
            || current_index.y >= i32(octreeMetaData.voxel_brick_dim)
            || current_index.z < 0
            || current_index.z >= i32(octreeMetaData.voxel_brick_dim)
        {
            return BrickHit(false, vec3u());
        }

        let bitmap_position_full_resolution = vec3u(
            vec3f(current_index) * (4. / f32(octreeMetaData.voxel_brick_dim))
        );
        if(
            bitmap_position_full_resolution.x != prev_bitmap_position_full_resolution.x
            || bitmap_position_full_resolution.y != prev_bitmap_position_full_resolution.y
            || bitmap_position_full_resolution.z != prev_bitmap_position_full_resolution.z
        ) {
            prev_bitmap_position_full_resolution = bitmap_position_full_resolution;
            mapped_index = flat_projection(vec3u(bitmap_position_full_resolution), vec2u(4, 4));
            if (
                0 == (
                    RAY_TO_LEAF_OCCUPANCY_BITMASK_LUT[mapped_index][direction_lut_index * 2]
                    & occupancy_bitmap_lsb
                )
                && 0 == (
                    RAY_TO_LEAF_OCCUPANCY_BITMASK_LUT[mapped_index][direction_lut_index * 2 + 1]
                    & occupancy_bitmap_msb
                )
            ){
                return BrickHit(false, vec3u());
            }
        }

        mapped_index = u32(flat_projection(
            vec3u(current_index),
            vec2u(octreeMetaData.voxel_brick_dim, octreeMetaData.voxel_brick_dim)
        ));
        if (brick_index_start + mapped_index) >= arrayLength(&voxels)
        {
            return BrickHit(false, vec3u());
        }
        if !is_empty(voxels[brick_index_start + mapped_index])
        {
            return BrickHit(true, vec3u(current_index));
        }

        let step = dda_step_to_next_sibling(
            ray,
            ray_current_distance,
            current_bounds,
            ray_scale_factors
        );
        current_bounds.min_position = (
            current_bounds.min_position
            + vec3f(step) * (brick_bounds.size / f32(octreeMetaData.voxel_brick_dim))
        );
        current_index = current_index + vec3i(round(step));
    }

    // Technically this line is unreachable
    return BrickHit(false, vec3u(0));
}

struct OctreeRayIntersection {
    hit: bool,
    albedo : vec4<f32>,
    content: u32,
    collision_point: vec3f,
    impact_normal: vec3f,
}

fn get_by_ray(ray: Line) -> OctreeRayIntersection{
    let ray_scale_factors = get_dda_scale_factors(ray);
    let direction_lut_index = hash_direction(ray.direction);

    var node_stack: array<u32, NODE_STACK_SIZE>;
    var node_stack_meta: u32 = 0;
    var current_bounds = Cube(vec3(0.), f32(octreeMetaData.octree_size));
    var ray_current_distance: f32  = 0.0;
    var target_octant = OOB_OCTANT;
    var current_node_key = EMPTY_MARKER;
    var step_vec = vec3f(0.);

    if(cube_intersect_ray(current_bounds, ray).hit){
        ray_current_distance = impact_or(cube_intersect_ray(current_bounds, ray), 0.);
        target_octant = hash_region(
            point_in_ray_at_distance(ray, ray_current_distance) - current_bounds.min_position,
            round(current_bounds.size / 2.),
        );
    }

    while target_octant != OOB_OCTANT {
        current_node_key = OCTREE_ROOT_NODE_KEY;
        current_bounds = Cube(vec3(0.), f32(octreeMetaData.octree_size));
        node_stack_push(&node_stack, &node_stack_meta, OCTREE_ROOT_NODE_KEY);
        while(!node_stack_is_empty(node_stack_meta)) {
            var leaf_miss = false;
            if is_leaf(nodes[current_node_key].sized_node_meta) {
                let leaf_brick_hit = traverse_brick(
                    ray, &ray_current_distance, nodes[current_node_key].voxels_start_at,
                    children_buffer[nodes[current_node_key].children_starts_at],
                    children_buffer[nodes[current_node_key].children_starts_at + 1],
                    current_bounds, ray_scale_factors, direction_lut_index
                );
                if leaf_brick_hit.hit == true {
                    let hit_in_voxels = (
                        nodes[current_node_key].voxels_start_at
                        + u32(flat_projection(
                            leaf_brick_hit.index,
                            vec2u(octreeMetaData.voxel_brick_dim, octreeMetaData.voxel_brick_dim)
                        ))
                    );
                    current_bounds.size = round(current_bounds.size / f32(octreeMetaData.voxel_brick_dim));
                    current_bounds.min_position = current_bounds.min_position
                        + vec3f(leaf_brick_hit.index) * current_bounds.size;
                    return OctreeRayIntersection(
                        true,
                        color_palette[voxels[hit_in_voxels].albedo_index],
                        voxels[hit_in_voxels].content,
                        point_in_ray_at_distance(ray, ray_current_distance),
                        cube_impact_normal(current_bounds, point_in_ray_at_distance(ray, ray_current_distance))
                    );
                }
                leaf_miss = true;
            }

            if( leaf_miss
                || target_octant == OOB_OCTANT
                || EMPTY_MARKER == current_node_key // Should never happen
                || 0 == get_node_occupancy_bitmap(nodes[current_node_key].sized_node_meta)
                || ( 0 == (
                    get_node_occupancy_bitmap(nodes[current_node_key].sized_node_meta)
                    & RAY_TO_NODE_OCCUPANCY_BITMASK_LUT[target_octant][direction_lut_index]
                ))
            ){
                // POP
                node_stack_pop(&node_stack, &node_stack_meta);
                step_vec = dda_step_to_next_sibling(
                    ray,
                    &ray_current_distance,
                    current_bounds,
                    ray_scale_factors
                );
                if(EMPTY_MARKER != node_stack_last(node_stack_meta)){
                    current_node_key = node_stack[node_stack_last(node_stack_meta)];
                    target_octant = step_octant(
                        hash_region( // parent current target octant
                            // current bound center
                            current_bounds.min_position + vec3f(round(current_bounds.size / 2.))
                            - ( // parent bound min position
                                current_bounds.min_position
                                - (current_bounds.min_position % (current_bounds.size * 2.))
                            ),
                            current_bounds.size
                        ),
                        step_vec
                    );
                    current_bounds.size = round(current_bounds.size * 2.);
                    current_bounds.min_position -= current_bounds.min_position % current_bounds.size;
                }
                continue;
            }

            var target_bounds = child_bounds_for(current_bounds, target_octant);
            var target_child_key = children_buffer[nodes[current_node_key].children_starts_at + target_octant];
            if (
                target_child_key < arrayLength(&nodes) //!crate::object_pool::key_is_valid
                && 0 != (
                    get_node_occupancy_bitmap(nodes[current_node_key].sized_node_meta)
                    & ( // crate::spatial::math::octant_bitmask
                        0x00000001u << (target_octant & 0x000000FF)
                    )
                )
            ) {
                // PUSH
                current_node_key = target_child_key;
                current_bounds = target_bounds;
                target_octant = hash_region( // child_target_octant
                    (point_in_ray_at_distance(ray, ray_current_distance) - target_bounds.min_position),
                    round(target_bounds.size / 2.)
                );
                node_stack_push(&node_stack, &node_stack_meta, target_child_key);
            } else {
                // ADVANCE
                loop {
                    step_vec = dda_step_to_next_sibling(
                        ray,
                        &ray_current_distance,
                        target_bounds,
                        ray_scale_factors
                    );
                    target_octant = step_octant(target_octant, step_vec);
                    if OOB_OCTANT != target_octant {
                        target_bounds = child_bounds_for(current_bounds, target_octant);
                        target_child_key = children_buffer[
                            nodes[current_node_key].children_starts_at + target_octant
                        ];
                    }

                    if (
                        target_octant == OOB_OCTANT
                        || (
                            target_child_key < arrayLength(&nodes) //crate::object_pool::key_is_valid
                            && 0 != (
                                get_node_occupancy_bitmap(nodes[current_node_key].sized_node_meta)
                                & (0x00000001u << target_octant) // crate::spatial::math::octant_bitmask
                            )
                            && 0 != (
                                get_node_occupancy_bitmap(nodes[target_child_key].sized_node_meta)
                                & RAY_TO_NODE_OCCUPANCY_BITMASK_LUT[hash_region(
                                    point_in_ray_at_distance(ray, ray_current_distance) - target_bounds.min_position,
                                    round(target_bounds.size / 2.)
                                )][direction_lut_index]
                            )
                        )
                    ) {
                        break;
                    }
                }
            }
        } // while (node_stack not empty)

        let current_octant_center = (
            current_bounds.min_position
            + vec3f(round(current_bounds.size / 2.))
            + step_vec * current_bounds.size
        );
        if(
          current_octant_center.x < f32(octreeMetaData.octree_size)
          && current_octant_center.y < f32(octreeMetaData.octree_size)
          && current_octant_center.z < f32(octreeMetaData.octree_size)
          && current_octant_center.x > 0.
          && current_octant_center.y > 0.
          && current_octant_center.z > 0.
        ) {
            target_octant = hash_region(
                current_octant_center, f32(octreeMetaData.octree_size) / 2.
            );
        } else {
            target_octant = OOB_OCTANT;
        }
    } // while (ray inside root bounds)
    return OctreeRayIntersection(false, vec4f(0.), 0, vec3f(0.), vec3f(0., 0., 1.));
}

struct Voxelement {
    albedo_index: u32,
    content: u32,
}

fn is_empty(e: Voxelement) -> bool {
    return (
        0. == color_palette[e.albedo_index].r
        && 0. == color_palette[e.albedo_index].g
        && 0. == color_palette[e.albedo_index].b
        && 0. == color_palette[e.albedo_index].a
        && 0 == e.content
    );
}

struct SizedNode {
    sized_node_meta: u32,
    children_starts_at: u32,
    voxels_start_at: u32,
}

const OCTREE_ROOT_NODE_KEY = 0u;
struct OctreeMetaData {
    ambient_light_color: vec3f,
    ambient_light_position: vec3f,
    octree_size: u32,
    voxel_brick_dim: u32,
}

struct Viewport {
    origin: vec3f,
    direction: vec3f,
    w_h_fov: vec3f,
}

@group(0) @binding(0)
var output_texture: texture_storage_2d<rgba8unorm, read_write>;

@group(0) @binding(1)
var<uniform> viewport: Viewport;

@group(1) @binding(0)
var<uniform> octreeMetaData: OctreeMetaData;

@group(1) @binding(1)
var<storage, read_write> nodes: array<SizedNode>;

@group(1) @binding(2)
var<storage, read_write> children_buffer: array<u32>;

@group(1) @binding(3)
var<storage, read_write> voxels: array<Voxelement>;

@group(1) @binding(4)
var<storage, read_write> color_palette: array<vec4f>;

@compute @workgroup_size(8, 8, 1)
fn update(
    @builtin(global_invocation_id) invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let ray_endpoint =
        (
            viewport.origin
            + (viewport.direction * viewport.w_h_fov.z)
            - (
                normalize(cross(vec3f(0., 1., 0.), viewport.direction))
                * (viewport.w_h_fov.x / 2.)
            )
            - (vec3f(0., 1., 0.) * (viewport.w_h_fov.y / 2.))
        ) // Viewport bottom left
        + (
            normalize(cross(vec3f(0., 1., 0.), viewport.direction))
            * viewport.w_h_fov.x
            * (f32(invocation_id.x) / f32(num_workgroups.x * 8))
        ) // Viewport right direction
        + (
            vec3f(0., 1., 0.) * viewport.w_h_fov.y
            * (1. - (f32(invocation_id.y) / f32(num_workgroups.y * 8)))
        ) // Viewport up direction
        ;
    var rgb_result = vec3f(0.5,0.5,0.5);
    var ray_result = get_by_ray(Line(ray_endpoint, normalize(ray_endpoint - viewport.origin)));
    if ray_result.hit == true {
        rgb_result = (
            ray_result.albedo.rgb * (
                dot(ray_result.impact_normal, vec3f(-0.5,0.5,-0.5)) / 2. + 0.5
            )
        ).rgb;
    }

    /*// +++ DEBUG +++
    // Display the xyz axes
    let root_hit = cube_intersect_ray(
        Cube(vec3(0.,0.,0.), f32(octreeMetaData.octree_size)), ray
    );
    if root_hit.hit == true {
        if root_hit. impact_hit == true {
            let axes_length = f32(octreeMetaData.octree_size) / 2.;
            let axes_width = f32(octreeMetaData.octree_size) / 50.;
            let entry_point = point_in_ray_at_distance(ray, root_hit.impact_distance);
            if entry_point.x < axes_length && entry_point.y < axes_width && entry_point.z < axes_width {
                rgb_result.r = 1.;
            }
            if entry_point.x < axes_width && entry_point.y < axes_length && entry_point.z < axes_width {
                rgb_result.g = 1.;
            }
            if entry_point.x < axes_width && entry_point.y < axes_width && entry_point.z < axes_length {
                rgb_result.b = 1.;
            }
        }

    }
    */// --- DEBUG ---

    textureStore(output_texture, vec2u(invocation_id.xy), vec4f(rgb_result, 1.));
}

// Note: should be const
var<private> OCTANT_STEP_RESULT_LUT: array<array<array<u32, 3>, 3>, 3> = array<array<array<u32, 3>, 3>, 3>(
    array<array<u32, 3>, 3>(
        array<u32, 3>(143165576,671647880,2284357768),
        array<u32, 3>(1216874632,1749559304,2288551976),
        array<u32, 3>(2290632840,2290640968,2290649192)
    ),
    array<array<u32, 3>, 3>(
        array<u32, 3>(277383304,839944328,2285013128),
        array<u32, 3>(1418203272,1985229328,2289469490),
        array<u32, 3>(2290635912,2290644564,2290649206)
    ),
    array<array<u32, 3>, 3>(
        array<u32, 3>(2173208712,2206304392,2290321544),
        array<u32, 3>(2240315784,2273674113,2290583683),
        array<u32, 3>(2290648456,2290648965,2290649223)
    )
);

// Note: should be const
var<private> RAY_TO_NODE_OCCUPANCY_BITMASK_LUT: array<array<u32, 8>, 8> = array<array<u32, 8>, 8>(
    array<u32, 8>(1, 3, 5, 15, 17, 51, 85, 255),
    array<u32, 8>(3, 2, 15, 10, 51, 34, 255, 170),
    array<u32, 8>(5, 15, 4, 12, 85, 255, 68, 204),
    array<u32, 8>(15, 10, 12, 8, 255, 170, 204, 136),
    array<u32, 8>(17, 51, 85, 255, 16, 48, 80, 240),
    array<u32, 8>(51, 34, 255, 170, 48, 32, 240, 160),
    array<u32, 8>(85, 255, 68, 204, 80, 240, 64, 192),
    array<u32, 8>(255, 170, 204, 136, 240, 160, 192, 128),
);

// Note: should be const
var<private> RAY_TO_LEAF_OCCUPANCY_BITMASK_LUT: array<array<u32, 16>, 64> = array<array<u32, 16>, 64>(
    array<u32, 16>(1,0,15,0,65537,65537,983055,983055,4369,0,65535,0,286331153,286331153,4294967295,4294967295,),
    array<u32, 16>(3,0,14,0,196611,196611,917518,917518,13107,0,61166,0,858993459,858993459,4008636142,4008636142,),
    array<u32, 16>(7,0,12,0,458759,458759,786444,786444,30583,0,52428,0,2004318071,2004318071,3435973836,3435973836,),
    array<u32, 16>(15,0,8,0,983055,983055,524296,524296,65535,0,34952,0,4294967295,4294967295,2290649224,2290649224,),
    array<u32, 16>(17,0,255,0,1114129,1114129,16711935,16711935,4368,0,65520,0,286265616,286265616,4293984240,4293984240,),
    array<u32, 16>(51,0,238,0,3342387,3342387,15597806,15597806,13104,0,61152,0,858796848,858796848,4007718624,4007718624,),
    array<u32, 16>(119,0,204,0,7798903,7798903,13369548,13369548,30576,0,52416,0,2003859312,2003859312,3435187392,3435187392,),
    array<u32, 16>(255,0,136,0,16711935,16711935,8913032,8913032,65520,0,34944,0,4293984240,4293984240,2290124928,2290124928,),
    array<u32, 16>(273,0,4095,0,17891601,17891601,268374015,268374015,4352,0,65280,0,285217024,285217024,4278255360,4278255360,),
    array<u32, 16>(819,0,3822,0,53674803,53674803,250482414,250482414,13056,0,60928,0,855651072,855651072,3993038336,3993038336,),
    array<u32, 16>(1911,0,3276,0,125241207,125241207,214699212,214699212,30464,0,52224,0,1996519168,1996519168,3422604288,3422604288,),
    array<u32, 16>(4095,0,2184,0,268374015,268374015,143132808,143132808,65280,0,34816,0,4278255360,4278255360,2281736192,2281736192,),
    array<u32, 16>(4369,0,65535,0,286331153,286331153,4294967295,4294967295,4096,0,61440,0,268439552,268439552,4026593280,4026593280,),
    array<u32, 16>(13107,0,61166,0,858993459,858993459,4008636142,4008636142,12288,0,57344,0,805318656,805318656,3758153728,3758153728,),
    array<u32, 16>(30583,0,52428,0,2004318071,2004318071,3435973836,3435973836,28672,0,49152,0,1879076864,1879076864,3221274624,3221274624,),
    array<u32, 16>(65535,0,34952,0,4294967295,4294967295,2290649224,2290649224,61440,0,32768,0,4026593280,4026593280,2147516416,2147516416,),
    array<u32, 16>(65537,0,983055,0,65536,65537,983040,983055,286331153,0,4294967295,0,286326784,286331153,4294901760,4294967295,),
    array<u32, 16>(196611,0,917518,0,196608,196611,917504,917518,858993459,0,4008636142,0,858980352,858993459,4008574976,4008636142,),
    array<u32, 16>(458759,0,786444,0,458752,458759,786432,786444,2004318071,0,3435973836,0,2004287488,2004318071,3435921408,3435973836,),
    array<u32, 16>(983055,0,524296,0,983040,983055,524288,524296,4294967295,0,2290649224,0,4294901760,4294967295,2290614272,2290649224,),
    array<u32, 16>(1114129,0,16711935,0,1114112,1114129,16711680,16711935,286265616,0,4293984240,0,286261248,286265616,4293918720,4293984240,),
    array<u32, 16>(3342387,0,15597806,0,3342336,3342387,15597568,15597806,858796848,0,4007718624,0,858783744,858796848,4007657472,4007718624,),
    array<u32, 16>(7798903,0,13369548,0,7798784,7798903,13369344,13369548,2003859312,0,3435187392,0,2003828736,2003859312,3435134976,3435187392,),
    array<u32, 16>(16711935,0,8913032,0,16711680,16711935,8912896,8913032,4293984240,0,2290124928,0,4293918720,4293984240,2290089984,2290124928,),
    array<u32, 16>(17891601,0,268374015,0,17891328,17891601,268369920,268374015,285217024,0,4278255360,0,285212672,285217024,4278190080,4278255360,),
    array<u32, 16>(53674803,0,250482414,0,53673984,53674803,250478592,250482414,855651072,0,3993038336,0,855638016,855651072,3992977408,3993038336,),
    array<u32, 16>(125241207,0,214699212,0,125239296,125241207,214695936,214699212,1996519168,0,3422604288,0,1996488704,1996519168,3422552064,3422604288,),
    array<u32, 16>(268374015,0,143132808,0,268369920,268374015,143130624,143132808,4278255360,0,2281736192,0,4278190080,4278255360,2281701376,2281736192,),
    array<u32, 16>(286331153,0,4294967295,0,286326784,286331153,4294901760,4294967295,268439552,0,4026593280,0,268435456,268439552,4026531840,4026593280,),
    array<u32, 16>(858993459,0,4008636142,0,858980352,858993459,4008574976,4008636142,805318656,0,3758153728,0,805306368,805318656,3758096384,3758153728,),
    array<u32, 16>(2004318071,0,3435973836,0,2004287488,2004318071,3435921408,3435973836,1879076864,0,3221274624,0,1879048192,1879076864,3221225472,3221274624,),
    array<u32, 16>(4294967295,0,2290649224,0,4294901760,4294967295,2290614272,2290649224,4026593280,0,2147516416,0,4026531840,4026593280,2147483648,2147516416,),
    array<u32, 16>(65537,1,983055,15,0,65537,0,983055,286331153,4369,4294967295,65535,0,286331153,0,4294967295,),
    array<u32, 16>(196611,3,917518,14,0,196611,0,917518,858993459,13107,4008636142,61166,0,858993459,0,4008636142,),
    array<u32, 16>(458759,7,786444,12,0,458759,0,786444,2004318071,30583,3435973836,52428,0,2004318071,0,3435973836,),
    array<u32, 16>(983055,15,524296,8,0,983055,0,524296,4294967295,65535,2290649224,34952,0,4294967295,0,2290649224,),
    array<u32, 16>(1114129,17,16711935,255,0,1114129,0,16711935,286265616,4368,4293984240,65520,0,286265616,0,4293984240,),
    array<u32, 16>(3342387,51,15597806,238,0,3342387,0,15597806,858796848,13104,4007718624,61152,0,858796848,0,4007718624,),
    array<u32, 16>(7798903,119,13369548,204,0,7798903,0,13369548,2003859312,30576,3435187392,52416,0,2003859312,0,3435187392,),
    array<u32, 16>(16711935,255,8913032,136,0,16711935,0,8913032,4293984240,65520,2290124928,34944,0,4293984240,0,2290124928,),
    array<u32, 16>(17891601,273,268374015,4095,0,17891601,0,268374015,285217024,4352,4278255360,65280,0,285217024,0,4278255360,),
    array<u32, 16>(53674803,819,250482414,3822,0,53674803,0,250482414,855651072,13056,3993038336,60928,0,855651072,0,3993038336,),
    array<u32, 16>(125241207,1911,214699212,3276,0,125241207,0,214699212,1996519168,30464,3422604288,52224,0,1996519168,0,3422604288,),
    array<u32, 16>(268374015,4095,143132808,2184,0,268374015,0,143132808,4278255360,65280,2281736192,34816,0,4278255360,0,2281736192,),
    array<u32, 16>(286331153,4369,4294967295,65535,0,286331153,0,4294967295,268439552,4096,4026593280,61440,0,268439552,0,4026593280,),
    array<u32, 16>(858993459,13107,4008636142,61166,0,858993459,0,4008636142,805318656,12288,3758153728,57344,0,805318656,0,3758153728,),
    array<u32, 16>(2004318071,30583,3435973836,52428,0,2004318071,0,3435973836,1879076864,28672,3221274624,49152,0,1879076864,0,3221274624,),
    array<u32, 16>(4294967295,65535,2290649224,34952,0,4294967295,0,2290649224,4026593280,61440,2147516416,32768,0,4026593280,0,2147516416,),
    array<u32, 16>(65537,65537,983055,983055,0,65536,0,983040,286331153,286331153,4294967295,4294967295,0,286326784,0,4294901760,),
    array<u32, 16>(196611,196611,917518,917518,0,196608,0,917504,858993459,858993459,4008636142,4008636142,0,858980352,0,4008574976,),
    array<u32, 16>(458759,458759,786444,786444,0,458752,0,786432,2004318071,2004318071,3435973836,3435973836,0,2004287488,0,3435921408,),
    array<u32, 16>(983055,983055,524296,524296,0,983040,0,524288,4294967295,4294967295,2290649224,2290649224,0,4294901760,0,2290614272,),
    array<u32, 16>(1114129,1114129,16711935,16711935,0,1114112,0,16711680,286265616,286265616,4293984240,4293984240,0,286261248,0,4293918720,),
    array<u32, 16>(3342387,3342387,15597806,15597806,0,3342336,0,15597568,858796848,858796848,4007718624,4007718624,0,858783744,0,4007657472,),
    array<u32, 16>(7798903,7798903,13369548,13369548,0,7798784,0,13369344,2003859312,2003859312,3435187392,3435187392,0,2003828736,0,3435134976,),
    array<u32, 16>(16711935,16711935,8913032,8913032,0,16711680,0,8912896,4293984240,4293984240,2290124928,2290124928,0,4293918720,0,2290089984,),
    array<u32, 16>(17891601,17891601,268374015,268374015,0,17891328,0,268369920,285217024,285217024,4278255360,4278255360,0,285212672,0,4278190080,),
    array<u32, 16>(53674803,53674803,250482414,250482414,0,53673984,0,250478592,855651072,855651072,3993038336,3993038336,0,855638016,0,3992977408,),
    array<u32, 16>(125241207,125241207,214699212,214699212,0,125239296,0,214695936,1996519168,1996519168,3422604288,3422604288,0,1996488704,0,3422552064,),
    array<u32, 16>(268374015,268374015,143132808,143132808,0,268369920,0,143130624,4278255360,4278255360,2281736192,2281736192,0,4278190080,0,2281701376,),
    array<u32, 16>(286331153,286331153,4294967295,4294967295,0,286326784,0,4294901760,268439552,268439552,4026593280,4026593280,0,268435456,0,4026531840,),
    array<u32, 16>(858993459,858993459,4008636142,4008636142,0,858980352,0,4008574976,805318656,805318656,3758153728,3758153728,0,805306368,0,3758096384,),
    array<u32, 16>(2004318071,2004318071,3435973836,3435973836,0,2004287488,0,3435921408,1879076864,1879076864,3221274624,3221274624,0,1879048192,0,3221225472,),
    array<u32, 16>(4294967295,4294967295,2290649224,2290649224,0,4294901760,0,2290614272,4026593280,4026593280,2147516416,2147516416,0,4026531840,0,2147483648,),
);
