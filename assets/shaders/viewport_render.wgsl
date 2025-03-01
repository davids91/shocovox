// The time since startup data is in the globals binding which is part of the mesh_view_bindings import
#import bevy_pbr::{
    mesh_view_bindings::globals,
    forward_io::VertexOutput
}

struct Line {
    origin: vec3f,
    direction: vec3f,
}

struct Cube {
    min_position: vec3f,
    size: f32,
}

const FLOAT_ERROR_TOLERANCE = 0.00001;
const OOB_OCTANT = 8u;
const COLOR_FOR_NODE_REQUEST_SENT = vec3f(0.5,0.3,0.0);
const COLOR_FOR_NODE_REQUEST_FAIL = vec3f(0.7,0.2,0.0);
const COLOR_FOR_BRICK_REQUEST_SENT = vec3f(0.3,0.1,0.0);
const COLOR_FOR_BRICK_REQUEST_FAIL = vec3f(0.6,0.0,0.0);

//crate::spatial::math::hash_region
fn hash_region(offset: vec3f, size_half: f32) -> u32 {
    return u32(offset.x >= size_half)
        + u32(offset.z >= size_half) * 2u
        + u32(offset.y >= size_half) * 4u;
}

//crate::spatial::mod::Cube::child_bounds_for
fn child_bounds_for(bounds: ptr<function, Cube>, octant: u32) -> Cube{
    return Cube(
        (*bounds).min_position + (OCTANT_OFFSET_REGION_LUT[octant] * (*bounds).size / 2.),
        round((*bounds).size / 2.)
    );
}

struct CubeRayIntersection {
    hit: bool,
    impact_hit: bool,
    impact_distance: f32,
    exit_distance: f32,
}

//crate::spatial::raytracing::Ray::point_at
fn point_in_ray_at_distance(ray: ptr<function, Line>, d: f32) -> vec3f{
    return (*ray).origin + (*ray).direction * d;
}

//crate::spatial::raytracing::Cube::intersect_ray
fn cube_intersect_ray(cube: Cube, ray: ptr<function, Line>,) -> CubeRayIntersection{
    let max_position = cube.min_position + vec3f(cube.size, cube.size, cube.size);
    let tmin = max(
        max(
            min(
                (cube.min_position.x - (*ray).origin.x) / (*ray).direction.x,
                (max_position.x - (*ray).origin.x) / (*ray).direction.x
            ),
            min(
                (cube.min_position.y - (*ray).origin.y) / (*ray).direction.y,
                (max_position.y - (*ray).origin.y) / (*ray).direction.y
            )
        ),
        min(
            (cube.min_position.z - (*ray).origin.z) / (*ray).direction.z,
            (max_position.z - (*ray).origin.z) / (*ray).direction.z
        )
    );
    let tmax = min(
        min(
            max(
                (cube.min_position.x - (*ray).origin.x) / (*ray).direction.x,
                (max_position.x - (*ray).origin.x) / (*ray).direction.x
            ),
            max(
                (cube.min_position.y - (*ray).origin.y) / (*ray).direction.y,
                (max_position.y - (*ray).origin.y) / (*ray).direction.y
            )
        ),
        max(
            (cube.min_position.z - (*ray).origin.z) / (*ray).direction.z,
            (max_position.z - (*ray).origin.z) / (*ray).direction.z
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


//crate::raytracing::NodeStack
const NODE_STACK_SIZE: u32 = 4;
const EMPTY_MARKER: u32 = 0xFFFFFFFFu;

//crate::raytracing::NodeStack::is_empty
fn node_stack_is_empty(node_stack_meta: u32) -> bool {
    return 0 == (node_stack_meta & 0x000000FFu);
}

//crate::raytracing::NodeStack::push
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


//crate::raytracing::NodeStack::pop
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

//crate::raytracing::NodeStack::last/last_mut
fn node_stack_last(node_stack_meta: u32) -> u32 { // returns either with index or EMPTY_MARKER
    if 0 == (node_stack_meta & 0x000000FFu) {
        return EMPTY_MARKER;
    }
    return (node_stack_meta & 0x0000FF00u) >> 8u;
}

//crate::octree:raytracing::get_dda_scale_factors
fn get_dda_scale_factors(ray: ptr<function, Line>) -> vec3f {
    return vec3f(
        sqrt(
            1.
            + pow((*ray).direction.z / (*ray).direction.x, 2.)
            + pow((*ray).direction.y / (*ray).direction.x, 2.)
        ),
        sqrt(
            pow((*ray).direction.x / (*ray).direction.y, 2.)
            + 1.
            + pow((*ray).direction.z / (*ray).direction.y, 2.)
        ),
        sqrt(
            pow((*ray).direction.x / (*ray).direction.z, 2.)
            + pow((*ray).direction.y / (*ray).direction.z, 2.)
            + 1.
        ),
    );
}

//crate::raytracing::dda_step_to_next_sibling
fn dda_step_to_next_sibling(
    ray: ptr<function, Line>,
    ray_current_distance: ptr<function,f32>,
    current_bounds: ptr<function, Cube>,
    ray_scale_factors: ptr<function, vec3f>
) -> vec3f {
    let d = (
        vec3f(*ray_current_distance, *ray_current_distance, *ray_current_distance) 
        + abs(
            ( // steps needed to reach next axis
                ((*current_bounds).size * max(sign((*ray).direction), vec3f(0.,0.,0.)))
                - (
                    sign((*ray).direction)
                    * (
                        point_in_ray_at_distance(ray, *ray_current_distance)
                        - (*current_bounds).min_position
                    )
                )
            )
            * *ray_scale_factors
        )
    );
    *ray_current_distance = min(d.x, min(d.y, d.z));

    var result = vec3f(0., 0., 0.);
    if abs(*ray_current_distance - d.x) < FLOAT_ERROR_TOLERANCE {
        result.x = sign((*ray).direction).x;
    }
    if abs(*ray_current_distance - d.y) < FLOAT_ERROR_TOLERANCE {
        result.y = sign((*ray).direction).y;
    }
    if abs(*ray_current_distance - d.z) < FLOAT_ERROR_TOLERANCE {
        result.z = sign((*ray).direction).z;
    }
    return result;
}

// Unique to this implementation, not adapted from rust code
/// Sets the used bit true for the given node
fn set_node_used(node_key: u32) {
    if 0 != (metadata[node_key] & 0x01u) {
        // no need to set if already true
        return;
    }

    loop{
        let exchange_result = atomicCompareExchangeWeak(
            &metadata[node_key], metadata[node_key], metadata[node_key] | 0x01u
        );
        if(exchange_result.exchanged || 0 < (exchange_result.old_value & 0x01u)){
            break;
        }
    }
}

// Unique to this implementation, not adapted from rust code
/// Sets the used bit true for the given brick
fn set_brick_used(brick_index: u32) {
    if 0 != ( metadata[brick_index / 8] & (0x01u << (24u + (brick_index % 8u))) ) {
        // no need to set if already true
        return;
    }

    loop{
        let exchange_result = atomicCompareExchangeWeak(
            &metadata[brick_index / 8],
            metadata[brick_index / 8],
            metadata[brick_index / 8] | (0x01u << (24u + (brick_index % 8u)))
        );
        if(
            exchange_result.exchanged
            || 0 != ( exchange_result.old_value & (0x01u << (24u + (brick_index % 8u))) )
        ){
            break;
        }
    }
}

// Unique to this implementation, not adapted from rust code
/// Requests the child of the given node to be uploaded
fn request_node(node_meta_index: u32, child_octant: u32) -> bool {
    var request_index = 0u;
    loop{
        let exchange_result = atomicCompareExchangeWeak(
            &node_requests[request_index], EMPTY_MARKER,
            (node_meta_index & 0x00FFFFFFu)|((child_octant & 0x000000FF) << 24)
        );
        if(
            exchange_result.exchanged 
            ||(
                exchange_result.old_value
                == ((node_meta_index & 0x00FFFFFFu)|((child_octant & 0x000000FF) << 24))
            )
        ) {
            break;
        }
        request_index += 1u;
        if(request_index >= arrayLength(&node_requests)) {
            return false;
        }
    }
    return true;
}

struct BrickHit{
    hit: bool,
    index: vec3u,
    flat_index: u32,
}

fn traverse_brick(
    ray: ptr<function, Line>,
    ray_current_distance: ptr<function,f32>,
    brick_start_index: u32,
    brick_bounds: ptr<function, Cube>,
    ray_scale_factors: ptr<function, vec3f>,
    direction_lut_index: u32,
) -> BrickHit {
    let dimension = i32(octree_meta_data.voxel_brick_dim);
    var current_index = clamp(
        vec3i(vec3f( // entry position in brick
            point_in_ray_at_distance(ray, *ray_current_distance)
            - (*brick_bounds).min_position
        ) * f32(dimension) / (*brick_bounds).size),
        vec3i(0),
        vec3i(dimension - 1)
    );
    var current_flat_index = (
        i32(brick_start_index) * (dimension * dimension * dimension)
        + ( //crate::spatial::math::flat_projection
            current_index.x
            + (current_index.y * dimension)
            + (current_index.z * dimension * dimension)
        )
    );
    var current_bounds = Cube(
        (
            (*brick_bounds).min_position 
            + vec3f(current_index) * round((*brick_bounds).size / f32(dimension))
        ),
        round((*brick_bounds).size / f32(dimension))
    );

    /*// +++ DEBUG +++
    var safety = 0u;
    */// --- DEBUG ---
    var step = vec3f(0.);
    loop{
        /*// +++ DEBUG +++
        safety += 1u;
        if(safety > u32(f32(dimension) * sqrt(30.))) {
            return BrickHit(false, vec3u(1, 1, 1), 0);
        }
        */// --- DEBUG ---
        if current_index.x < 0
            || current_index.x >= dimension
            || current_index.y < 0
            || current_index.y >= dimension
            || current_index.z < 0
            || current_index.z >= dimension
        {
            return BrickHit(false, vec3u(), 0);
        }


        // step delta calculated from crate::spatial::math::flat_projection
        // --> e.g. flat_delta_y = flat_projection(0, 1, 0, brick_dim);
        current_flat_index += (
            i32(step.x)
            + i32(step.y) * dimension
            + i32(step.z) * dimension * dimension
        );

        if current_flat_index >= i32(arrayLength(&voxels))
        {
            return BrickHit(false, vec3u(current_index), u32(current_flat_index));
        }
        if !is_empty(voxels[current_flat_index])
        {
            return BrickHit(true, vec3u(current_index), u32(current_flat_index));
        }

        step = round(dda_step_to_next_sibling(
            ray,
            ray_current_distance,
            &current_bounds,
            ray_scale_factors
        ));
        current_bounds.min_position += step * current_bounds.size;
        current_index += vec3i(step);
    }

    // Technically this line is unreachable
    return BrickHit(false, vec3u(0), 0);
}

struct OctreeRayIntersection {
    hit: bool,
    albedo : vec4<f32>,
    collision_point: vec3f,
    impact_normal: vec3f,
}

fn probe_brick(
    ray: ptr<function, Line>,
    ray_current_distance: ptr<function,f32>,
    leaf_node_key: u32,
    brick_octant: u32,
    brick_bounds: ptr<function, Cube>,
    ray_scale_factors: ptr<function, vec3f>,
    direction_lut_index: u32,
) -> OctreeRayIntersection {
    if( // node is occupied at target octant, meaning: brick is not empty
        0 != (
            BITMAP_MASK_FOR_OCTANT_LUT[brick_octant][0]
            & node_occupied_bits[leaf_node_key * 2]
        )
        || 0 != (
            BITMAP_MASK_FOR_OCTANT_LUT[brick_octant][1]
            & node_occupied_bits[leaf_node_key * 2 + 1]
        )
    ){
        let brick_index = node_children[((leaf_node_key * 8) + brick_octant)];
        set_brick_used(brick_index);
        if(0 == ((0x01u << (16 + brick_octant)) & metadata[leaf_node_key])) { // brick is solid
            // Whole brick is solid, ray hits it at first connection
            return OctreeRayIntersection(
                true,
                color_palette[brick_index & 0x0000FFFF], // Albedo is in color_palette, it's not a brick index in this case
                point_in_ray_at_distance(ray, *ray_current_distance),
                cube_impact_normal(*brick_bounds, point_in_ray_at_distance(ray, *ray_current_distance))
            );
        } else { // brick is parted
            let leaf_brick_hit = traverse_brick(
                ray, ray_current_distance,
                brick_index,
                brick_bounds, ray_scale_factors, direction_lut_index
            );
            if leaf_brick_hit.hit == true {
                return OctreeRayIntersection(
                    true,
                    color_palette[voxels[leaf_brick_hit.flat_index] & 0x0000FFFF],
                    point_in_ray_at_distance(ray, *ray_current_distance),
                    cube_impact_normal(
                        Cube(
                            (*brick_bounds).min_position + (
                                vec3f(leaf_brick_hit.index)
                                * round((*brick_bounds).size / f32(octree_meta_data.voxel_brick_dim))
                            ),
                            round((*brick_bounds).size / f32(octree_meta_data.voxel_brick_dim)),
                        ),
                        point_in_ray_at_distance(ray, *ray_current_distance)
                    )
                );
            }
        }
    }
    return OctreeRayIntersection(false, vec4f(0.), vec3f(0.), vec3f(0., 0., 1.));
}

fn probe_MIP(
    ray: ptr<function, Line>,
    ray_current_distance: ptr<function,f32>,
    node_key: u32,
    node_bounds: ptr<function, Cube>,
    ray_scale_factors: ptr<function, vec3f>,
    direction_lut_index: u32,
) -> OctreeRayIntersection {
    if( // there is a valid mip present
        0 != (metadata[node_key] & 0x00000010) // node has MIP
        && node_mips[node_key] != EMPTY_MARKER // which is uploaded
    ) {
        if(0 == (metadata[node_key] & 0x00000020)) { // MIP brick is solid
            // Whole brick is solid, ray hits it at first connection
            return OctreeRayIntersection(
                true,
                color_palette[node_mips[node_key] & 0x0000FFFF], // Albedo is in color_palette, it's not a brick index in this case
                point_in_ray_at_distance(ray, *ray_current_distance),
                cube_impact_normal((*node_bounds), point_in_ray_at_distance(ray, *ray_current_distance))
            );
        } else { // brick is parted
            set_brick_used(node_mips[node_key]);
            let original_distance = *ray_current_distance;
            let leaf_brick_hit = traverse_brick(
                ray, ray_current_distance,
                node_mips[node_key],
                node_bounds, ray_scale_factors, direction_lut_index
            );
            let final_distance = *ray_current_distance;
            *ray_current_distance = original_distance;
            if leaf_brick_hit.hit == true {
                return OctreeRayIntersection(
                    true,
                    color_palette[voxels[leaf_brick_hit.flat_index] & 0x0000FFFF],
                    point_in_ray_at_distance(ray, final_distance),
                    cube_impact_normal(
                        Cube(
                            (*node_bounds).min_position + (
                                vec3f(leaf_brick_hit.index)
                                * round((*node_bounds).size / f32(octree_meta_data.voxel_brick_dim))
                            ),
                            round((*node_bounds).size / f32(octree_meta_data.voxel_brick_dim)),
                        ),
                        point_in_ray_at_distance(ray, final_distance)
                    )
                );
            }
        }
    }
    return OctreeRayIntersection(false, vec4f(0.), vec3f(0.), vec3f(0., 0., 1.));
}

// Unique to this implementation, not adapted from rust code
/// Traverses the node to provide information about how the occupied bits of the node
/// and the given ray collides. The higher the number, the closer the hit is.
fn traverse_node_for_ocbits(
    ray: ptr<function, Line>,
    ray_current_distance: ptr<function,f32>,
    node_key: u32,
    node_bounds: ptr<function, Cube>,
    ray_scale_factors: ptr<function, vec3f>,
) -> f32 {
    let original_distance = *ray_current_distance;

    var position = vec3f(
        point_in_ray_at_distance(ray, *ray_current_distance)
        - (*node_bounds).min_position
    );

    var current_index = vec3i(vec3f(
        clamp( (position.x * 4. / (*node_bounds).size), 0.01, 3.99),
        clamp( (position.y * 4. / (*node_bounds).size), 0.01, 3.99),
        clamp( (position.z * 4. / (*node_bounds).size), 0.01, 3.99),
    ));

    var current_bounds = Cube(
        (
            (*node_bounds).min_position
            + vec3f(current_index) * ((*node_bounds).size / 4.)
        ),
        round((*node_bounds).size / 4.)
    );

    var safety = 0u;
    var result = 0.;
    loop {
        if safety > 10 || current_index.x < 0 || current_index.x >= 4
            || current_index.y < 0 || current_index.y >= 4
            || current_index.z < 0 || current_index.z >= 4
        {
            break;
        }

        let bitmap_index = BITMAP_INDEX_LUT[u32(current_index.x)]
                                           [u32(current_index.y)]
                                           [u32(current_index.z)];
        if (
            (
                (bitmap_index < 32)
                && (0u != (node_occupied_bits[node_key * 2]
                            & (0x01u << bitmap_index) ))
            )||(
                (bitmap_index >= 32)
                && (0u != (node_occupied_bits[node_key * 2 + 1]
                            & (0x01u << (bitmap_index - 32)) ))
            )
        ){
            result = 1. - (f32(safety) * 0.25);
            break;
        }

        let step = round(dda_step_to_next_sibling(
            ray,
            ray_current_distance,
            &current_bounds,
            ray_scale_factors
        ));
        current_bounds.min_position += step * current_bounds.size;
        current_index += vec3i(step);
        safety += 1u;
    }

    *ray_current_distance = original_distance;
    return result;
}

fn get_by_ray(ray: ptr<function, Line>) -> OctreeRayIntersection {
    var ray_scale_factors = get_dda_scale_factors(ray); // Should be const, but then it can't be passed as ptr
    let direction_lut_index = ( //crate::spatial::math::hash_direction
        hash_region(vec3f(1.) + normalize((*ray).direction), 1.)
    );

    var node_stack: array<u32, NODE_STACK_SIZE>;
    var node_stack_meta: u32 = 0;
    var ray_current_distance = 0.0;
    var current_bounds = Cube(vec3(0.), f32(octree_meta_data.octree_size));
    var current_node_key = EMPTY_MARKER;
    var current_node_meta = 0u;
    var target_octant = OOB_OCTANT;
    var step_vec = vec3f(0.);
    var missing_data_color = vec3f(0.);
    var mip_level = log2(f32(octree_meta_data.octree_size) / f32(octree_meta_data.voxel_brick_dim));

    let root_intersect = cube_intersect_ray(current_bounds, ray);
    if(root_intersect.hit){
        if(root_intersect.impact_hit) {
            ray_current_distance = root_intersect.impact_distance;
        }
        target_octant = hash_region(
            point_in_ray_at_distance(ray, ray_current_distance) - current_bounds.min_position,
            round(current_bounds.size / 2.),
        );
    }
    /*// +++ DEBUG +++
    var outer_safety = 0;
    */// --- DEBUG ---
    while target_octant != OOB_OCTANT {
        /*// +++ DEBUG +++
        outer_safety += 1;
        if(f32(outer_safety) > f32(octree_meta_data.octree_size) * sqrt(3.)) {
            return OctreeRayIntersection(
                true, vec4f(1.,0.,0.,1.), 0, vec3f(0.), vec3f(0., 0., 1.)
            );
        }
       */ // --- DEBUG ---
        current_node_key = OCTREE_ROOT_NODE_KEY;
        current_node_meta = metadata[OCTREE_ROOT_NODE_KEY];
        current_bounds = Cube(vec3(0.), f32(octree_meta_data.octree_size));
        node_stack_push(&node_stack, &node_stack_meta, OCTREE_ROOT_NODE_KEY);
        /*// +++ DEBUG +++
        var safety = 0;
        */// --- DEBUG ---
        while(!node_stack_is_empty(node_stack_meta)) {
            /*// +++ DEBUG +++
            safety += 1;
            if(f32(safety) > f32(octree_meta_data.octree_size) * sqrt(30.)) {
                return OctreeRayIntersection(
                    true, vec4f(0.,0.,1.,1.), 0, vec3f(0.), vec3f(0., 0., 1.)
                );
            }
            */// --- DEBUG ---
            // backtrack by default after miss, in case node is a uniform leaf
            var do_backtrack_after_leaf_miss = (0x0000000C == (0x0000000C & current_node_meta));
            var target_child_key = node_children[(current_node_key * 8) + target_octant];
            var target_bounds = child_bounds_for(&current_bounds, target_octant);
            var bitmap_pos_in_node = clamp(
                (
                    point_in_ray_at_distance(ray, ray_current_distance)
                    - current_bounds.min_position
                ) * 4. / current_bounds.size,
                vec3f(FLOAT_ERROR_TOLERANCE),
                vec3f(4. - FLOAT_ERROR_TOLERANCE)
            );

            /*// +++ DEBUG +++
            if(current_bounds.size == f32(octree_meta_data.octree_size / debug_data)) {
                missing_data_color += (
                    vec3f(0.0,0.0,0.7)
                    * vec3f(traverse_node_for_ocbits(
                        ray,
                        &ray_current_distance,
                        current_node_key,
                        &current_bounds,
                        &ray_scale_factors
                    ))
                );
            }
            */// --- DEBUG ---
            if( // In case current node MIP level is smaller, than the required MIP level
                mip_level <
                (
                    /*length( // based on ray current travel distance
                        viewport.origin - (
                            round(point_in_ray_at_distance(ray, ray_current_distance) / f32(current_bounds.size))
                            * f32(current_bounds.size) // rounded to bound border
                        )
                    )
                    / f32(debug_data)*/
                    length( // based on ray current travel distance
                        viewport.origin - (
                            round(point_in_ray_at_distance(ray, ray_current_distance) / (mip_level * 2.))
                            * (mip_level * 2.) // aligned to nearest cube edges(based on current MIP level)
                        )
                    )
                    / f32(viewport.frustum.z)
                )
            ){
                if( // node has MIP which is not uploaded
                    0 != (current_node_meta & 0x00000010)
                    && node_mips[current_node_key] == EMPTY_MARKER
                ){
                    request_node(current_node_key, OOB_OCTANT);
                } else {
                    let mip_hit = probe_MIP(
                        ray, &ray_current_distance,
                        current_node_key, &current_bounds,
                        &ray_scale_factors, direction_lut_index
                    );
                    if true == mip_hit.hit {
                        return mip_hit;
                    }
                }
            }

            /*// +++ DEBUG +++
            if(
                current_bounds.size == f32(
                    octree_meta_data.octree_size / debug_data
                )
            ){
                missing_data_color += (
                    vec3f(0.0,0.0,0.7)
                    * vec3f(traverse_node_for_ocbits(
                        ray,
                        &ray_current_distance,
                        current_node_key,
                        &current_bounds,
                        &ray_scale_factors
                    ))
                );
            }
            */// --- DEBUG ---

            if(
                // In case node doesn't yet have the target child node uploaded to GPU
                target_octant != OOB_OCTANT
                && target_child_key == EMPTY_MARKER // target child key is invalid
                && ( // node is occupied at target octant
                    0 != (
                        BITMAP_MASK_FOR_OCTANT_LUT[target_octant][0]
                        & node_occupied_bits[current_node_key * 2]
                    )
                    || 0 != (
                        BITMAP_MASK_FOR_OCTANT_LUT[target_octant][1]
                        & node_occupied_bits[current_node_key * 2 + 1]
                    )
                )
                // Request node only once per ray iteration to prioritize nodes in sight for cache
                //&& 0 == (missing_data_color.r + missing_data_color.g + missing_data_color.b)
            ){
                if request_node(current_node_key, target_octant) {
                    missing_data_color += COLOR_FOR_NODE_REQUEST_SENT;
                } else {
                    missing_data_color += COLOR_FOR_NODE_REQUEST_FAIL;
                }
                // Since a node have just been requested, display MIP if available
                let MIP_hit = probe_MIP(
                    ray, &ray_current_distance,
                    current_node_key, &current_bounds,
                    &ray_scale_factors, direction_lut_index
                );
                if true == MIP_hit.hit {
                    return MIP_hit;
                }
            } else
            // +++ DEBUG +++
            /*if(
                0 != (current_node_meta & 0x00000010) // node has MIP
                && node_mips[current_node_key] == EMPTY_MARKER // which is not uploaded
            ){
                request_node(current_node_key, OOB_OCTANT);
            }
            else if( // Debug interface selected this level and there is a valid mip present
                (current_bounds.size == f32(octree_meta_data.octree_size / debug_data))
                && 0 != (current_node_meta & 0x00000010) // node has MIP
                && node_mips[current_node_key] != EMPTY_MARKER // which is uploaded
            ) {
                let MIP_hit = probe_MIP(
                    ray, &ray_current_distance,
                    current_node_key, &current_bounds,
                    &ray_scale_factors, direction_lut_index
                );
                if true == MIP_hit.hit {
                    return MIP_hit;
                }
            } else*/
            // --- DEBUG ---

            if(
                (target_octant != OOB_OCTANT)
                && (0 != (0x00000004 & current_node_meta)) // node is leaf
            ){
                    var hit: OctreeRayIntersection;
                    if(0 != (0x00000008 & current_node_meta)) { // node is a uniform leaf
                        hit = probe_brick(
                            ray, &ray_current_distance,
                            current_node_key, 0u, &current_bounds,
                            &ray_scale_factors, direction_lut_index
                        );
                        do_backtrack_after_leaf_miss = true;
                    } else { // node is a non-uniform leaf
                        target_bounds = child_bounds_for(&current_bounds, target_octant);
                        hit = probe_brick(
                            ray, &ray_current_distance,
                            current_node_key, target_octant,
                            &target_bounds,
                            &ray_scale_factors, direction_lut_index
                        );
                    }
                    if hit.hit == true {
                        hit.albedo -= vec4f(missing_data_color, 0.);

                        /*// +++ DEBUG +++
                        let current_point = point_in_ray_at_distance(ray, ray_current_distance);
                        let bound_size_ratio = f32(current_bounds.size) / f32(octree_meta_data.octree_size) * 5.;
                        if( // Display current bounds boundaries
                            (abs(current_point.x - current_bounds.min_position.x) < bound_size_ratio)
                            ||(abs(current_point.y - current_bounds.min_position.y) < bound_size_ratio)
                            ||(abs(current_point.z - current_bounds.min_position.z) < bound_size_ratio)
                        ){
                            hit.albedo -= 0.5;
                        }

                        /*if( // Display current bounds center
                            (abs(current_point.x - (current_bounds.min_position.x + (current_bounds.size / 2.))) < bound_size_ratio)
                            ||(abs(current_point.y - (current_bounds.min_position.y + (current_bounds.size / 2.))) < bound_size_ratio)
                            ||(abs(current_point.z - (current_bounds.min_position.z + (current_bounds.size / 2.))) < bound_size_ratio)
                        ){
                            hit.albedo += 0.5;
                        }*/
                        */// --- DEBUG ---
                        return hit;
                    }
                
            }

            if( do_backtrack_after_leaf_miss
                || target_octant == OOB_OCTANT
                || EMPTY_MARKER == current_node_key // Guards statements in other conditions, but should never happen
                || ( // There is no overlap in node occupancy and ray potential hit area
                    0 == (
                        RAY_TO_NODE_OCCUPANCY_BITMASK_LUT[
                            BITMAP_INDEX_LUT[u32(bitmap_pos_in_node.x)]
                                            [u32(bitmap_pos_in_node.y)]
                                            [u32(bitmap_pos_in_node.z)]
                        ][direction_lut_index * 2]
                        & node_occupied_bits[current_node_key * 2]
                    )
                    && 0 == (
                        RAY_TO_NODE_OCCUPANCY_BITMASK_LUT[
                            BITMAP_INDEX_LUT[u32(bitmap_pos_in_node.x)]
                                            [u32(bitmap_pos_in_node.y)]
                                            [u32(bitmap_pos_in_node.z)]
                        ][direction_lut_index * 2 + 1]
                        & node_occupied_bits[current_node_key * 2 + 1]
                    )
                )
            ) {
                // POP
                node_stack_pop(&node_stack, &node_stack_meta);
                step_vec = dda_step_to_next_sibling(
                    ray, &ray_current_distance,
                    &current_bounds,
                    &ray_scale_factors
                );
                mip_level += 1.;
                if(EMPTY_MARKER != node_stack_last(node_stack_meta)){
                    current_node_key = node_stack[node_stack_last(node_stack_meta)];
                    current_node_meta = metadata[current_node_key];

                    // target octant updated to hold the target octant of the parent node temporarily
                    target_octant = hash_region(
                        // current bound center
                        current_bounds.min_position + vec3f(round(current_bounds.size / 2.))
                        - ( // parent bound min position
                            current_bounds.min_position
                            - (current_bounds.min_position % (current_bounds.size * 2.))
                        ),
                        current_bounds.size
                    );

                    // ..which is then stepped forward as if the parent was advancing
                    target_octant = ( //crate::spatial::math::step_octant
                       (
                           OCTANT_STEP_RESULT_LUT[u32(sign(step_vec.x) + 1)]
                                                 [u32(sign(step_vec.y) + 1)]
                                                 [u32(sign(step_vec.z) + 1)]
                           & (0x0Fu << (4 * target_octant))
                       ) >> (4 * target_octant)
                    ) & 0x0Fu;
                    current_bounds.size = round(current_bounds.size * 2.);
                    current_bounds.min_position -= current_bounds.min_position % current_bounds.size;
                }
                continue;
            }

            if (
                (
                    0 == (0x00000004 & current_node_meta) // node is not a leaf
                    && target_child_key != EMPTY_MARKER //crate::object_pool::key_is_valid
                )
                && ( // There is overlap in node occupancy and potential ray hit area
                    0 != (
                        RAY_TO_NODE_OCCUPANCY_BITMASK_LUT[
                            BITMAP_INDEX_LUT[u32(bitmap_pos_in_node.x)]
                                            [u32(bitmap_pos_in_node.y)]
                                            [u32(bitmap_pos_in_node.z)]
                        ][direction_lut_index * 2]
                        & node_occupied_bits[current_node_key * 2]
                    )
                    || 0 != (
                        RAY_TO_NODE_OCCUPANCY_BITMASK_LUT[
                            BITMAP_INDEX_LUT[u32(bitmap_pos_in_node.x)]
                                            [u32(bitmap_pos_in_node.y)]
                                            [u32(bitmap_pos_in_node.z)]
                        ][direction_lut_index * 2 + 1]
                        & node_occupied_bits[current_node_key * 2 + 1]
                    )
                )
            ) {
                // PUSH
                set_node_used(target_child_key);
                current_node_key = target_child_key;
                current_node_meta = metadata[current_node_key];
                current_bounds = target_bounds;
                target_octant = hash_region( // child_target_octant
                    (point_in_ray_at_distance(ray, ray_current_distance) - target_bounds.min_position),
                    round(target_bounds.size / 2.)
                );
                node_stack_push(&node_stack, &node_stack_meta, target_child_key);
                mip_level -= 1.; 
            } else {
                // ADVANCE
                /*// +++ DEBUG +++
                var advance_safety = 0;
                */// --- DEBUG ---
                loop {
                    /*// +++ DEBUG +++
                    advance_safety += 1;
                    if(advance_safety > 4) {
                        return OctreeRayIntersection(
                            true, vec4f(1.,0.,1.,1.), 0, vec3f(0.), vec3f(0., 0., 1.)
                        );
                    }
                    */// --- DEBUG ---
                    step_vec = round(dda_step_to_next_sibling(
                        ray, &ray_current_distance,
                        &target_bounds,
                        &ray_scale_factors
                    ));
                    target_octant = ( //crate::spatial::math::step_octant
                       (
                           OCTANT_STEP_RESULT_LUT[u32(sign(step_vec.x) + 1)]
                                                 [u32(sign(step_vec.y) + 1)]
                                                 [u32(sign(step_vec.z) + 1)]
                           & (0x0Fu << (4 * target_octant))
                       ) >> (4 * target_octant)
                    ) & 0x0Fu;
                    if OOB_OCTANT != target_octant {
                        target_bounds = child_bounds_for(&current_bounds, target_octant);
                        target_child_key = node_children[(current_node_key * 8) + target_octant];
                        bitmap_pos_in_node += step_vec * 4. / current_bounds.size;
                        if(
                            target_child_key == EMPTY_MARKER // target child key is invalid
                            && ( // node is occupied at target octant
                                0 != (
                                    BITMAP_MASK_FOR_OCTANT_LUT[target_octant][0]
                                    & node_occupied_bits[current_node_key * 2]
                                )
                                || 0 != (
                                    BITMAP_MASK_FOR_OCTANT_LUT[target_octant][1]
                                    & node_occupied_bits[current_node_key * 2 + 1]
                                )
                            )
                        ){
                            if request_node(current_node_key, target_octant) {
                                missing_data_color += (
                                    COLOR_FOR_NODE_REQUEST_SENT
                                    * vec3f(traverse_node_for_ocbits(
                                        ray,
                                        &ray_current_distance,
                                        current_node_key,
                                        &current_bounds,
                                        &ray_scale_factors
                                    ))
                                );
                            } else {
                                missing_data_color += (
                                    COLOR_FOR_NODE_REQUEST_FAIL
                                    * vec3f(traverse_node_for_ocbits(
                                        ray,
                                        &ray_current_distance,
                                        current_node_key,
                                        &current_bounds,
                                        &ray_scale_factors
                                    ))
                                );
                            }
                        }
                    }
                    if (
                        target_octant == OOB_OCTANT
                        || ( // In case the current internal node has a valid target child
                            target_child_key != EMPTY_MARKER //crate::object_pool::key_is_valid
                            && 0 == (0x00000004 & current_node_meta) // node is not a leaf
                            && ( // target child is in the area the ray can potentially hit
                                0 != (
                                    RAY_TO_NODE_OCCUPANCY_BITMASK_LUT[
                                        BITMAP_INDEX_LUT[u32(bitmap_pos_in_node.x)]
                                                        [u32(bitmap_pos_in_node.y)]
                                                        [u32(bitmap_pos_in_node.z)]
                                    ][direction_lut_index * 2]
                                    & node_occupied_bits[current_node_key * 2]
                                )
                                || 0 != (
                                    RAY_TO_NODE_OCCUPANCY_BITMASK_LUT[
                                        BITMAP_INDEX_LUT[u32(bitmap_pos_in_node.x)]
                                                        [u32(bitmap_pos_in_node.y)]
                                                        [u32(bitmap_pos_in_node.z)]
                                    ][direction_lut_index * 2 + 1]
                                    & node_occupied_bits[current_node_key * 2 + 1]
                                )
                            )
                        )
                        || ( // In case the current node is leaf and its target brick is not empty
                            (0 != (0x00000004 & current_node_meta))
                            && ( // node is occupied at target octant
                                0 != (
                                    BITMAP_MASK_FOR_OCTANT_LUT[target_octant][0]
                                    & node_occupied_bits[current_node_key * 2]
                                )
                                || 0 != (
                                    BITMAP_MASK_FOR_OCTANT_LUT[target_octant][1]
                                    & node_occupied_bits[current_node_key * 2 + 1]
                                )
                            )
                        )
                    ) {
                        break;
                    }
                }
            }
        } // while (node_stack not empty)

        let next_octant_center = (
            current_bounds.min_position
            + vec3f(round(current_bounds.size / 2.))
            + step_vec * current_bounds.size
        );
        if(
          next_octant_center.x < f32(octree_meta_data.octree_size)
          && next_octant_center.y < f32(octree_meta_data.octree_size)
          && next_octant_center.z < f32(octree_meta_data.octree_size)
          && next_octant_center.x > 0.
          && next_octant_center.y > 0.
          && next_octant_center.z > 0.
        ) {
            target_octant = hash_region(
                next_octant_center, f32(octree_meta_data.octree_size) / 2.
            );
        } else {
            target_octant = OOB_OCTANT;
        }
        // Push ray current distance a little bit forward to avoid iterating the same paths all over again
        ray_current_distance += FLOAT_ERROR_TOLERANCE;
    } // while (ray inside root bounds)
    return OctreeRayIntersection(false, vec4f(missing_data_color, 1.), vec3f(0.), vec3f(0., 0., 1.));
}

alias PaletteIndexValues = u32;

fn is_empty(e: PaletteIndexValues) -> bool {
    return (
        (0x0000FFFF == (0x0000FFFF & e))
        ||(
            0. == color_palette[e & 0x0000FFFF].a
            && 0. == color_palette[e & 0x0000FFFF].r
            && 0. == color_palette[e & 0x0000FFFF].g
            && 0. == color_palette[e & 0x0000FFFF].b
        )
    );
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
    frustum: vec3f,
    fov: f32,
}

@group(0) @binding(0)
var output_texture: texture_storage_2d<rgba8unorm, read_write>;

@group(0) @binding(1)
var<uniform> viewport: Viewport;

@group(0) @binding(2)
var<storage, read_write> node_requests: array<atomic<u32>>;

@group(0) @binding(3)
var<uniform> debug_data: u32;

@group(1) @binding(0)
var<uniform> octree_meta_data: OctreeMetaData;

@group(1) @binding(1)
var<storage, read_write> metadata: array<atomic<u32>>;

@group(1) @binding(2)
var<storage, read_write> node_children: array<u32>;

@group(1) @binding(3)
var<storage, read_write> node_mips: array<u32>;

@group(1) @binding(4)
var<storage, read_write> node_occupied_bits: array<u32>;

@group(1) @binding(5)
var<storage, read_write> voxels: array<PaletteIndexValues>;

@group(1) @binding(6)
var<storage, read_write> color_palette: array<vec4f>;


@compute @workgroup_size(8, 8, 1)
fn update(
    @builtin(global_invocation_id) invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let ray_endpoint =
        (
            viewport.origin
            + (viewport.direction * viewport.fov)
            - (
                normalize(cross(vec3f(0., 1., 0.), viewport.direction))
                * (viewport.frustum.x / 2.)
            )
            - (vec3f(0., 1., 0.) * (viewport.frustum.y / 2.))
        ) // Viewport bottom left
        + (
            normalize(cross(vec3f(0., 1., 0.), viewport.direction))
            * viewport.frustum.x
            * (f32(invocation_id.x) / f32(num_workgroups.x * 8))
        ) // Viewport right direction
        + (
            vec3f(0., 1., 0.) * viewport.frustum.y
            * (1. - (f32(invocation_id.y) / f32(num_workgroups.y * 8)))
        ) // Viewport up direction
        ;
    var ray = Line(ray_endpoint, normalize(ray_endpoint - viewport.origin));
    var rgb_result = vec3f(0.5,0.5,0.5);
    var ray_result = get_by_ray(&ray);
    if ray_result.hit == true {
        rgb_result = (
            ray_result.albedo.rgb * (
                dot(ray_result.impact_normal, vec3f(-0.5,0.5,-0.5)) / 2. + 0.5
            )
        ).rgb;
    } else {
        rgb_result = (rgb_result + ray_result.albedo.rgb) / 2.;
    }

    /*// +++ DEBUG +++
    var root_bounds = Cube(vec3(0.,0.,0.), f32(octree_meta_data.octree_size));
    let root_intersect = cube_intersect_ray(root_bounds, &ray);
    if root_intersect.hit == true {
        // Display the xyz axes
        if root_intersect. impact_hit == true {
            let axes_length = f32(octree_meta_data.octree_size) / 2.;
            let axes_width = f32(octree_meta_data.octree_size) / 50.;
            let entry_point = point_in_ray_at_distance(&ray, root_intersect.impact_distance);
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
        rgb_result.b += 0.1; // Also color in the area of the octree
    }
    */// --- DEBUG ---
    textureStore(output_texture, vec2u(invocation_id.xy), vec4f(rgb_result, 1.));
}

//crate::spatial::math::offset_region
var<private> OCTANT_OFFSET_REGION_LUT: array<vec3f, 8> = array<vec3f, 8>(
    vec3f(0., 0., 0.), vec3f(1., 0., 0.), vec3f(0., 0., 1.), vec3f(1., 0., 1.),
    vec3f(0., 1., 0.), vec3f(1., 1., 0.), vec3f(0., 1., 1.), vec3f(1.,1.,1.)
);
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
var<private> BITMAP_INDEX_LUT: array<array<array<u32, 4>, 4>, 4> = array<array<array<u32, 4>, 4>, 4>(
    array<array<u32, 4>, 4>(
        array<u32, 4>(0,16,32,48,),
        array<u32, 4>(4,20,36,52,),
        array<u32, 4>(8,24,40,56,),
        array<u32, 4>(12,28,44,60,),
    ),
    array<array<u32, 4>, 4>(
        array<u32, 4>(1,17,33,49,),
        array<u32, 4>(5,21,37,53,),
        array<u32, 4>(9,25,41,57,),
        array<u32, 4>(13,29,45,61,),
    ),
    array<array<u32, 4>, 4>(
        array<u32, 4>(2,18,34,50,),
        array<u32, 4>(6,22,38,54,),
        array<u32, 4>(10,26,42,58,),
        array<u32, 4>(14,30,46,62,),
    ),
    array<array<u32, 4>, 4>(
        array<u32, 4>(3,19,35,51,),
        array<u32, 4>(7,23,39,55,),
        array<u32, 4>(11,27,43,59,),
        array<u32, 4>(15,31,47,63,),
    ),
);

//crate::spatial::math::mask_for_octant_64_bits
var<private> BITMAP_MASK_FOR_OCTANT_LUT: array<array<u32, 2>, 8> = array<array<u32, 2>, 8>(
    array<u32, 2>(0x00330033u,0x00000000u),
    array<u32, 2>(0x00CC00CCu,0x00000000u),
    array<u32, 2>(0x00000000u,0x00330033u),
    array<u32, 2>(0x00000000u,0x00CC00CCu),
    array<u32, 2>(0x33003300u,0x00000000u),
    array<u32, 2>(0xCC00CC00u,0x00000000u),
    array<u32, 2>(0x00000000u,0x33003300u),
    array<u32, 2>(0x00000000u,0xCC00CC00u),
);

// Note: should be const
var<private> RAY_TO_NODE_OCCUPANCY_BITMASK_LUT: array<array<u32, 16>, 64> = array<array<u32, 16>, 64>(
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
