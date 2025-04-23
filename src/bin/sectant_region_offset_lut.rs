use shocovox_rs::octree::V3c;

pub(crate) const BOX_NODE_DIMENSION: usize = 4;
pub(crate) const BOX_NODE_CHILDREN_COUNT: usize = 64;

pub(crate) fn flat_projection(x: usize, y: usize, z: usize, size: usize) -> usize {
    x + (y * size) + (z * size * size)
}

/// Internal utility for generating Lookup tables
/// Generates the relative position of each sectant inside a BoxNode
fn main() {
    let mut sectant_offset = [V3c::unit(0.); BOX_NODE_CHILDREN_COUNT];

    for x in 0..BOX_NODE_DIMENSION {
        for y in 0..BOX_NODE_DIMENSION {
            for z in 0..BOX_NODE_DIMENSION {
                let target_flat_index = flat_projection(x, y, z, BOX_NODE_DIMENSION);
                sectant_offset[target_flat_index] = V3c::new(
                    x as f32 / BOX_NODE_DIMENSION as f32,
                    y as f32 / BOX_NODE_DIMENSION as f32,
                    z as f32 / BOX_NODE_DIMENSION as f32,
                );
            }
        }
    }
    println!("CPU LUT: {:?}", sectant_offset);
    println!("WGSL LUT:");
    println!(
        "//const\nvar<private> SECTANT_OFFSET_REGION_LUT: array<vec3f, {}> = array<u32, {}>(",
        BOX_NODE_CHILDREN_COUNT, BOX_NODE_CHILDREN_COUNT
    );

    for (sectant, offset) in sectant_offset.iter().enumerate() {
        if 0 == (sectant % 4) && 0 != sectant {
            println!();
        }
        if 0 == (sectant % 16) && 0 != sectant {
            println!();
        }
        if 0 == (sectant % 4) {
            print!("\t");
        }
        print!("vec3f({:?}, {:?}, {:?}),", offset.x, offset.y, offset.z);
    }
    println!("\n);");

    #[rustfmt::skip]
    let _sectant_offset = [
        V3c { x: 0.0, y: 0.0, z: 0.0 }, V3c { x: 0.25, y: 0.0, z: 0.0 }, V3c { x: 0.5, y: 0.0, z: 0.0 }, V3c { x: 0.75, y: 0.0, z: 0.0 },
        V3c { x: 0.0, y: 0.25, z: 0.0 }, V3c { x: 0.25, y: 0.25, z: 0.0 }, V3c { x: 0.5, y: 0.25, z: 0.0 }, V3c { x: 0.75, y: 0.25, z: 0.0 },
        V3c { x: 0.0, y: 0.5, z: 0.0 }, V3c { x: 0.25, y: 0.5, z: 0.0 }, V3c { x: 0.5, y: 0.5, z: 0.0 }, V3c { x: 0.75, y: 0.5, z: 0.0 },
        V3c { x: 0.0, y: 0.75, z: 0.0 }, V3c { x: 0.25, y: 0.75, z: 0.0 }, V3c { x: 0.5, y: 0.75, z: 0.0 }, V3c { x: 0.75, y: 0.75, z: 0.0 },

        V3c { x: 0.0, y: 0.0, z: 0.25 }, V3c { x: 0.25, y: 0.0, z: 0.25 }, V3c { x: 0.5, y: 0.0, z: 0.25 }, V3c { x: 0.75, y: 0.0, z: 0.25 },
        V3c { x: 0.0, y: 0.25, z: 0.25 }, V3c { x: 0.25, y: 0.25, z: 0.25 }, V3c { x: 0.5, y: 0.25, z: 0.25 }, V3c { x: 0.75, y: 0.25, z: 0.25 },
        V3c { x: 0.0, y: 0.5, z: 0.25 }, V3c { x: 0.25, y: 0.5, z: 0.25 }, V3c { x: 0.5, y: 0.5, z: 0.25 }, V3c { x: 0.75, y: 0.5, z: 0.25 },
        V3c { x: 0.0, y: 0.75, z: 0.25 }, V3c { x: 0.25, y: 0.75, z: 0.25 }, V3c { x: 0.5, y: 0.75, z: 0.25 }, V3c { x: 0.75, y: 0.75, z: 0.25 },

        V3c { x: 0.0, y: 0.0, z: 0.5 }, V3c { x: 0.25, y: 0.0, z: 0.5 }, V3c { x: 0.5, y: 0.0, z: 0.5 }, V3c { x: 0.75, y: 0.0, z: 0.5 },
        V3c { x: 0.0, y: 0.25, z: 0.5 }, V3c { x: 0.25, y: 0.25, z: 0.5 }, V3c { x: 0.5, y: 0.25, z: 0.5 }, V3c { x: 0.75, y: 0.25, z: 0.5 },
        V3c { x: 0.0, y: 0.5, z: 0.5 }, V3c { x: 0.25, y: 0.5, z: 0.5 }, V3c { x: 0.5, y: 0.5, z: 0.5 }, V3c { x: 0.75, y: 0.5, z: 0.5 },
        V3c { x: 0.0, y: 0.75, z: 0.5 }, V3c { x: 0.25, y: 0.75, z: 0.5 }, V3c { x: 0.5, y: 0.75, z: 0.5 }, V3c { x: 0.75, y: 0.75, z: 0.5 },

        V3c { x: 0.0, y: 0.0, z: 0.75 }, V3c { x: 0.25, y: 0.0, z: 0.75 }, V3c { x: 0.5, y: 0.0, z: 0.75 }, V3c { x: 0.75, y: 0.0, z: 0.75 },
        V3c { x: 0.0, y: 0.25, z: 0.75 }, V3c { x: 0.25, y: 0.25, z: 0.75 }, V3c { x: 0.5, y: 0.25, z: 0.75 }, V3c { x: 0.75, y: 0.25, z: 0.75 },
        V3c { x: 0.0, y: 0.5, z: 0.75 }, V3c { x: 0.25, y: 0.5, z: 0.75 }, V3c { x: 0.5, y: 0.5, z: 0.75 }, V3c { x: 0.75, y: 0.5, z: 0.75 },
        V3c { x: 0.0, y: 0.75, z: 0.75 }, V3c { x: 0.25, y: 0.75, z: 0.75 }, V3c { x: 0.5, y: 0.75, z: 0.75 }, V3c { x: 0.75, y: 0.75, z: 0.75 }
    ];
}
