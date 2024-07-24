use crate::octree::{Albedo, Octree, V3c, VoxelData};
use dot_vox::{Color, DotVoxData, Model, SceneNode};

impl From<Albedo> for Color {
    fn from(color: Albedo) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    }
}

impl From<Color> for Albedo {
    fn from(color: Color) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    }
}

impl VoxelData for Color {
    fn new(albedo: Albedo, _: u32) -> Self {
        albedo.into()
    }
    fn albedo(&self) -> Albedo {
        (*self).into()
    }
    fn user_data(&self) -> u32 {
        0
    }
    fn clear(&mut self) {
        self.r = 0;
        self.g = 0;
        self.b = 0;
        self.a = 0;
    }
}

fn iterate_vox_tree<F: FnMut(&Model, &V3c<i32>) -> ()>(vox_tree: &DotVoxData, mut fun: F) {
    let mut node_stack: Vec<(u32, V3c<i32>, u32)> = Vec::new();
    let current_position = V3c::new(0, 0, 0);

    match &vox_tree.scenes[0] {
        SceneNode::Transform {
            attributes: _,
            frames: _,
            child,
            layer_id: _,
        } => {
            // let transform_position: Vec<i32> = frames[0]
            //     .attributes
            //     .get("_t")
            //     .unwrap(
            //     .split(" ")
            //     .map(|x| x.parse().expect("Not an integer!"))
            //     .collect();
            // node_stack.push((*child, transform_position.into(), 0));
            node_stack.push((*child, V3c::unit(0), 0));
        }
        _ => {
            panic!("The root node for a magicka voxel DAG should be a transform")
        }
    }

    while 0 < node_stack.len() {
        let (current_node, transform, index) = *node_stack.last().unwrap();
        match &vox_tree.scenes[current_node as usize] {
            SceneNode::Transform {
                attributes: _,
                frames,
                child,
                layer_id: _,
            } => {
                let transform_delta: V3c<i32> = frames[0]
                    .attributes
                    .get("_t")
                    .unwrap()
                    .split(" ")
                    .map(|x| x.parse().expect("Not an integer!"))
                    .collect::<Vec<i32>>()
                    .into();
                // 0 == index ==> iterate into the child of the transform
                if 0 == index {
                    node_stack.push((*child, transform + transform_delta, 0));
                } else {
                    // 0 != index ==> remove transform and iterate into parent
                    node_stack.pop();
                    if let Some(parent) = node_stack.last_mut() {
                        parent.1 = parent.1 - transform_delta;
                    }
                }
            }
            SceneNode::Group {
                attributes: _,
                children,
            } => {
                if (index as usize) < children.len() {
                    node_stack.last_mut().unwrap().2 += 1;
                    node_stack.push((children[index as usize], transform, 0));
                } else {
                    node_stack.pop();
                    if let Some(parent) = node_stack.last_mut() {
                        parent.2 += 1;
                    }
                }
            }
            SceneNode::Shape {
                attributes: _,
                models,
            } => {
                for model in models {
                    fun(&vox_tree.models[model.model_id as usize], &current_position);
                }
                node_stack.pop();
                if let Some(parent) = node_stack.last_mut() {
                    parent.2 += 1;
                }
            }
        }
    }
}

impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Eq + Clone + Copy + VoxelData,
{
    pub fn load_magica_voxel_file(filename: &str) -> Result<Self, &'static str> {
        let vox_tree = dot_vox::load(filename)?;
        // println!("vox tree scenes: {:?}", vox_tree.scenes);
        // let mut i = 0;
        // for s in vox_tree.scenes {
        //     println!("[{}] {:?}", i, s);
        //     i += 1;
        // }
        // panic!("AH");

        let mut min_position = V3c::new(0, 0, 0);
        let mut max_position = V3c::new(0, 0, 0);
        iterate_vox_tree(&vox_tree, |model, position| {
            // println!("model_position: {:?}", position);
            min_position.x = min_position.x.min(position.x);
            min_position.y = min_position.y.min(position.y);
            min_position.z = min_position.z.min(position.z);
            if (position.x + model.size.x as i32) > max_position.x
                && (position.y + model.size.y as i32) > max_position.y
                && (position.z + model.size.z as i32) > max_position.z
            {
                max_position.x = position.x + model.size.x as i32;
                max_position.y = position.y + model.size.y as i32;
                max_position.z = position.z + model.size.z as i32;
            }
        });
        max_position = max_position - min_position;
        println!("max_position: {:?}", max_position);
        println!("min position: {:?}", min_position);
        let max_dimension = max_position.x.max(max_position.y).max(max_position.z);
        let max_dimension = (max_dimension as f32).log2().ceil() as u32;
        let max_dimension = 2_u32.pow(max_dimension);
        println!("octree size: {max_dimension}");
        let mut shocovox_octree = Octree::<T, DIM>::new(max_dimension).ok().unwrap();
        iterate_vox_tree(&vox_tree, |model, position| {
            let current_position = V3c::<u32>::from(*position - V3c::<i32>::from(min_position));
            for v in &model.voxels {
                shocovox_octree
                    .insert(
                        &(current_position + V3c::new(v.x as u32, v.y as u32, v.z as u32)),
                        T::new(vox_tree.palette[v.i as usize].into(), 0),
                    )
                    .ok();
            }
        });
        println!("Tree built form model!");
        Ok(shocovox_octree)
    }
}
