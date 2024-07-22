struct Voxelement {
    albedo : vec4<f32>,
    content: u32,
}

struct SizedNode {
    sized_node_meta: u32,
    children_starts_at: u32,
    voxels_start_at: u32,
}

const OCTREE_ROOT_NODE_KEY = 0u;
struct OctreeMetaData {
    octree_size: u32,
    voxel_brick_dim: u32,
    ambient_light_color: vec4f,
    ambient_light_position: vec3f,
}

struct Viewport {
    origin: vec3f,
    direction: vec3f,
    w_h_fov: vec3f,
}

@group(0) @binding(0)
var output_texture: texture_storage_2d<rgba8unorm, read_write>;

@group(0) @binding(1)
var output_texture_render: texture_2d<f32>;

@group(0) @binding(2)
var output_texture_sampler: sampler;

@group(0) @binding(3)
var<uniform> viewport: Viewport;

@group(1) @binding(0)
var<uniform> octreeMetaData: OctreeMetaData;

@group(1) @binding(1)
var<storage, read> nodes: array<SizedNode>;

@group(1) @binding(2)
var<storage, read> children_buffer: array<u32>;

@group(1) @binding(3)
var<storage, read> voxels: array<Voxelement>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    // map the input vertex index values to rectangle x,y coordinates
    var x = 0.; var y = 0.;
    if 0 == in_vertex_index || 3 == in_vertex_index {
        x = -1.;
        y = -1.;
    }else if 1 == in_vertex_index {
        x = -1.;
        y = 1.;
    }else if 2 == in_vertex_index || 4 == in_vertex_index {
        x = 1.;
        y = 1.;
    }else if 5 == in_vertex_index {
        x = 1.;
        y = -1.;
    }

    let pos = vec4f(x, y, 0.0, 1.0);
    let uv = vec2f((x + 1.) / 2.,(y + 1.) / 2.);
    return VertexOutput(pos,uv);
}

@fragment
fn fs_main(vertex_output: VertexOutput) -> @location(0) vec4<f32> {
    //return vec4f(vertex_output.uv, 0., 1.);
    return textureSample(
        output_texture_render, output_texture_sampler,
        vertex_output.uv
    );
}