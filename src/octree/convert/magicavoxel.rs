use crate::{
    octree::{Albedo, Octree, V3c, VoxelData},
    spatial::math::{convert_coordinate, CoordinateSystemType},
};
use dot_vox::{Color, DotVoxData, Model, SceneNode, Size, Voxel};
use nalgebra::Matrix3;

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

impl From<Voxel> for V3c<i32> {
    fn from(other: Voxel) -> Self {
        Self {
            x: other.x as i32,
            y: other.y as i32,
            z: other.z as i32,
        }
    }
}

impl From<Size> for V3c<i32> {
    fn from(other: Size) -> Self {
        Self {
            x: other.x as i32,
            y: other.y as i32,
            z: other.z as i32,
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

/// Converts the given byte value to a rotation matrix
/// Rotation matrix in voxel context enables 90 degr rotations only, so the contents of the matrix is restricted to 0,1,-1
fn parse_rotation_matrix(b: u8) -> Matrix3<i8> {
    let mut result = Matrix3::<i8>::new(0, 0, 0, 0, 0, 0, 0, 0, 0);

    // decide absolute values of each row
    let index_in_first_row = b & 0x3;
    let index_in_second_row = ((b & (0x3 << 2)) >> 2) & 0x3;
    let index_in_third_row = !(index_in_first_row ^ index_in_second_row) & 0x3;
    debug_assert!(index_in_first_row < 3);
    debug_assert!(index_in_second_row < 3);
    debug_assert!(index_in_third_row < 3);
    debug_assert!(index_in_first_row != index_in_second_row);
    debug_assert!(index_in_first_row != index_in_third_row);
    debug_assert!(index_in_second_row != index_in_third_row);

    // decide the sign of the values
    let sign_first_row = if 0 == (b & 0x10) { 1 } else { -1 };
    let sign_second_row = if 0 == (b & 0x20) { 1 } else { -1 };
    let sign_third_row = if 0 == (b & 0x40) { 1 } else { -1 };

    // set the values in the matrix
    result.data.0[0][index_in_first_row as usize] = sign_first_row;
    result.data.0[1][index_in_second_row as usize] = sign_second_row;
    result.data.0[2][index_in_third_row as usize] = sign_third_row;

    result
}

impl<T> V3c<T>
where
    T: num_traits::Num + Clone + Copy + std::convert::From<i8>,
{
    fn clone_transformed(&self, matrix: &Matrix3<i8>) -> V3c<T> {
        V3c::new(
            self.x * matrix.m11.into() + self.y * matrix.m21.into() + self.z * matrix.m31.into(),
            self.x * matrix.m21.into() + self.y * matrix.m22.into() + self.z * matrix.m23.into(),
            self.x * matrix.m31.into() + self.y * matrix.m32.into() + self.z * matrix.m33.into(),
        )
    }
}

fn iterate_vox_tree<F: FnMut(&Model, &V3c<i32>, &Matrix3<i8>) -> ()>(
    vox_tree: &DotVoxData,
    mut fun: F,
) {
    let mut node_stack: Vec<(u32, V3c<i32>, Matrix3<i8>, u32)> = Vec::new();

    match &vox_tree.scenes[0] {
        SceneNode::Transform {
            attributes: _,
            frames: _,
            child,
            layer_id: _,
        } => {
            node_stack.push((*child, V3c::unit(0), Matrix3::identity(), 0));
        }
        _ => {
            panic!("The root node for a magicka voxel DAG should be a translation")
        }
    }

    while 0 < node_stack.len() {
        let (current_node, translation, rotation, index) = *node_stack.last().unwrap();
        // println!("=========================================================");
        // println!("node_stack size: {}", node_stack.len());
        // println!(
        //     "node: {}, translation: {:?}, index: {}",
        //     current_node, translation, index
        // );
        match &vox_tree.scenes[current_node as usize] {
            SceneNode::Transform {
                attributes: _,
                frames,
                child,
                layer_id: _,
            } => {
                // println!("Processing translation");
                let translation = if let Some(t) = frames[0].attributes.get("_t") {
                    translation
                        + t.split(" ")
                            .map(|x| x.parse().expect("Not an integer!"))
                            .collect::<Vec<i32>>()
                            .into()
                } else {
                    translation
                };
                // println!("orientation: {:?}", rotation,);
                let orientation = if let Some(r) = frames[0].attributes.get("_r") {
                    // println!(
                    //     "orientation updated with: {:?}",
                    //     parse_rotation_matrix(r.parse().unwrap()),
                    // );
                    // println!(
                    //     "updated orientation: {:?}",
                    //     rotation * parse_rotation_matrix(r.parse().unwrap()),
                    // );
                    rotation
                        * parse_rotation_matrix(
                            r.parse()
                                .expect("Expected valid u8 byte to parse rotation matrix"),
                        )
                } else {
                    rotation
                };
                // 0 == index ==> iterate into the child of the translation
                if 0 == index {
                    node_stack.push((*child, translation, orientation, 0));
                } else {
                    // println!("pop");
                    // 0 != index ==> remove translation and iterate into parent
                    node_stack.pop();
                }
            }
            SceneNode::Group {
                attributes: _,
                children,
            } => {
                // println!("Processing Group[{index}] at {:?}", translation);
                if (index as usize) < children.len() {
                    node_stack.last_mut().unwrap().3 += 1;
                    node_stack.push((children[index as usize], translation, rotation, 0));
                } else {
                    node_stack.pop();
                    if let Some(parent) = node_stack.last_mut() {
                        parent.3 += 1;
                    }
                }
            }
            SceneNode::Shape {
                attributes: _,
                models,
            } => {
                // println!("Processing shape at {:?}", translation);
                for model in models {
                    fun(
                        &vox_tree.models[model.model_id as usize],
                        &translation,
                        &rotation,
                    );
                }
                node_stack.pop();
                if let Some(parent) = node_stack.last_mut() {
                    parent.3 += 1;
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
        // for s in &vox_tree.scenes {
        //     println!("[{}] {:?}", i, s);
        //     i += 1;
        // }
        // println!("vox tree model sizes: ");
        // i = 0;
        // for s in &vox_tree.models {
        //     println!("[{}] {:?}", i, s.size);
        //     i += 1;
        // }

        let mut min_position_lyup = V3c::<i32>::new(0, 0, 0);
        let mut max_position_lyup = V3c::<i32>::new(0, 0, 0);
        iterate_vox_tree(&vox_tree, |model, position, orientation| {
            // println!("raw pos: {:?}", position);
            let position = convert_coordinate(
                V3c::from(*position),
                CoordinateSystemType::RZUP,
                CoordinateSystemType::LYUP,
            );
            let model_size = convert_coordinate(
                V3c::from(model.size).clone_transformed(orientation),
                CoordinateSystemType::RZUP,
                CoordinateSystemType::LYUP,
            );
            // println!("orientation: {:?}", orientation);
            // println!("converted pos: {:?}", position);
            // println!(
            //     "model_size: {:?} --> {:?}",
            //     V3c::from(model.size),
            //     model_size
            // );
            min_position_lyup.x = min_position_lyup.x.min(position.x - (model_size.x / 2));
            min_position_lyup.y = min_position_lyup.y.min(position.y - (model_size.y / 2));
            min_position_lyup.z = min_position_lyup.z.min(position.z - (model_size.z / 2));
            // println!("min position --> {:?}", min_position);
            // println!("max pos: {:?} + {:?}", position, model_size);
            if (position.x + model_size.x) > max_position_lyup.x
                || (position.y + model_size.y) > max_position_lyup.y
                || (position.z + model_size.z) > max_position_lyup.z
            {
                max_position_lyup = position + model_size;
            }
            // println!("max_position: {:?}", max_position_lyup);
            // println!("min position: {:?}", min_position_lyup);
        });
        max_position_lyup = max_position_lyup - min_position_lyup;
        // println!("\nmax_position: {:?}", max_position_lyup);
        // println!("min position: {:?}", min_position_lyup);
        let max_dimension = max_position_lyup
            .x
            .max(max_position_lyup.y)
            .max(max_position_lyup.z);
        let max_dimension = (max_dimension as f32).log2().ceil() as u32;
        let max_dimension = 2_u32.pow(max_dimension);
        // println!("octree size: {max_dimension} \n ============================ \n ");
        let mut shocovox_octree = Octree::<T, DIM>::new(max_dimension).ok().unwrap();
        let mut vmin = V3c::unit(max_dimension as u32);
        let mut vmax = V3c::unit(0u32);
        iterate_vox_tree(&vox_tree, |model, position, orientation| {
            let model_size_half_lyup = convert_coordinate(
                V3c::from(model.size).clone_transformed(orientation),
                CoordinateSystemType::RZUP,
                CoordinateSystemType::LYUP,
            ) / 2;
            let position = V3c::from(*position);
            // println!("model_size: {:?}", model.size);
            // println!("model_size_half: {:?}", V3c::from(model.size) / 2);
            // println!(
            //     "m11: {:?}, m21: {:?}, m31: {:?}",
            //     orientation.m11, orientation.m21, orientation.m31
            // );
            // println!(
            //     "oriented model size: {:?}",
            //     V3c::from(model.size).clone_transformed(orientation)
            // );
            // println!("oriented model_size_half_lyup: {:?}", model_size_half_lyup);
            // println!("raw pos: {:?}", position);
            // println!(
            //     "pos_lyup: {:?}",
            //     convert_coordinate(
            //         position,
            //         CoordinateSystemType::RZUP,
            //         CoordinateSystemType::LYUP,
            //     )
            // );
            // println!(
            //     "corrected position_lyup: {:?}",
            //     convert_coordinate(
            //         position,
            //         CoordinateSystemType::RZUP,
            //         CoordinateSystemType::LYUP,
            //     ) - min_position_lyup
            //         - model_size_half_lyup
            // );
            let position_lyup = convert_coordinate(
                position,
                CoordinateSystemType::RZUP,
                CoordinateSystemType::LYUP,
            );
            let current_position = position_lyup - min_position_lyup - model_size_half_lyup;
            for voxel in &model.voxels {
                let voxel_position = convert_coordinate(
                    V3c::from(*voxel).clone_transformed(orientation),
                    CoordinateSystemType::RZUP,
                    CoordinateSystemType::LYUP,
                );
                // println!(
                //     "{:?} + {:?} - ({:?}/2) = {:?} ",
                //     current_position,
                //     voxel_position,
                //     model_size_half_lyup * 2,
                //     current_position + voxel_position
                // );
                let cpos = current_position + voxel_position;
                if cpos.length() < vmin.length() {
                    vmin = cpos.into();
                }
                if cpos.length() > vmax.length() {
                    vmax = cpos.into();
                }

                shocovox_octree
                    .insert(
                        &V3c::<u32>::from(current_position + voxel_position.into()),
                        T::new(vox_tree.palette[voxel.i as usize].into(), 0),
                    )
                    .ok()
                    .unwrap();
            }
            // println!("model position: {:?}", current_position);
            // println!("|min, max: {:?}, {:?}\n\n", vmin, vmax);
        });
        println!("Tree built from model!");
        Ok(shocovox_octree)
    }
}

#[cfg(test)]
mod octree_tests {
    use super::parse_rotation_matrix;
    use nalgebra::Matrix3;

    #[test]
    fn test_matrix_parse() {
        let test_matrix = Matrix3::<i8>::new(1, 0, 0, 0, 1, 0, 0, 0, 1);
        assert!(test_matrix.m11 == 1);
        assert!(test_matrix.m22 == 1);
        assert!(test_matrix.m33 == 1);

        let parsed_matrix = parse_rotation_matrix(4);
        assert!(parse_rotation_matrix(4) == test_matrix);
        assert!(parsed_matrix.m11 == 1);
        assert!(parsed_matrix.m22 == 1);
        assert!(parsed_matrix.m33 == 1);

        // https://github.com/ephtracy/voxel-model/blob/master/MagicaVoxel-file-format-vox-extension.txt
        // Matrix3 storage is column major
        let example = Matrix3::<i8>::new(0, 0, -1, 1, 0, 0, 0, -1, 0);
        assert!(
            parse_rotation_matrix((1 << 0) | (2 << 2) | (0 << 4) | (1 << 5) | (1 << 6)) == example
        );
        assert!(example.m21 == 1);
        assert!(example.m32 == -1);
        assert!(example.m13 == -1);
    }
}