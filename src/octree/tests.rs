mod brick_tests {
    use crate::{
        object_pool::empty_marker,
        octree::{flat_projection, types::PaletteIndexValues, Albedo, BrickData, NodeContent, V3c},
        spatial::lut::OCTANT_OFFSET_REGION_LUT,
    };

    #[test]
    fn test_octant_empty() {
        let color_palette = vec![Albedo::default().with_alpha(100); 1];
        let data_palette = vec![0u32; 1];
        let data = BrickData::<PaletteIndexValues>::Empty;
        assert!(data.is_empty_throughout(0, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(1, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(2, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(3, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(4, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(5, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(6, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(7, 1, &color_palette, &data_palette),);

        let data = BrickData::<PaletteIndexValues>::Parted(vec![NodeContent::pix_visual(0); 1]);
        assert!(!data.is_empty_throughout(0, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(1, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(2, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(3, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(4, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(5, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(6, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(7, 1, &color_palette, &data_palette),);
    }

    #[test]
    fn test_octant_empty_where_dim_is_2() {
        // Create a filled parted Brick
        let color_palette = vec![Albedo::default().with_alpha(100); 1];
        let data_palette = vec![0u32; 1];
        let mut data =
            BrickData::<PaletteIndexValues>::Parted(vec![NodeContent::pix_visual(0); 2 * 2 * 2]);
        assert!(!data.is_empty_throughout(0, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(1, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(2, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(3, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(4, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(5, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(6, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(7, 2, &color_palette, &data_palette),);

        // Erase an octant worth of data, it should be empty!
        let target_octant: u8 = 5;
        if let BrickData::Parted(ref mut brick) = data {
            let octant_offset =
                V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[target_octant as usize]);
            let octant_flat_offset =
                flat_projection(octant_offset.x, octant_offset.y, octant_offset.z, 2);
            brick[octant_flat_offset] = empty_marker::<PaletteIndexValues>();
        }
        assert!(
            data.is_empty_throughout(target_octant, 2, &color_palette, &data_palette,),
            "Data cleared under octant should be empty"
        );
    }

    #[test]
    fn test_octant_empty_where_dim_is_4() {
        // Create a filled parted Brick
        let color_palette = vec![Albedo::default().with_alpha(100); 1];
        let data_palette = vec![0u32; 1];
        let mut data =
            BrickData::<PaletteIndexValues>::Parted(vec![NodeContent::pix_visual(0); 4 * 4 * 4]);
        assert!(!data.is_empty_throughout(0, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(1, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(2, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(3, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(4, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(5, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(6, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(7, 4, &color_palette, &data_palette),);

        let target_octant: u8 = 5;
        // offset by half of the brick dimension, as half of the dim 4x4x4 translates to 2x2x2
        // which is the resolution of OCTANT_OFFSET_REGION_LUT
        let octant_offset =
            V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[target_octant as usize] * 2.);

        // Erase part of the octant, it should still not be empty
        if let BrickData::Parted(ref mut brick) = data {
            let octant_flat_offset =
                flat_projection(octant_offset.x, octant_offset.y, octant_offset.z, 4);
            brick[octant_flat_offset] = empty_marker::<PaletteIndexValues>();
        }
        assert!(
            !data.is_empty_throughout(target_octant, 4, &color_palette, &data_palette,),
            "Data cleared under octant should not be empty"
        );

        // Erase an octant worth of data, it should be empty!
        if let BrickData::Parted(ref mut brick) = data {
            for x in 0..2 {
                for y in 0..2 {
                    for z in 0..2 {
                        let octant_flat_offset = flat_projection(
                            octant_offset.x + x,
                            octant_offset.y + y,
                            octant_offset.z + z,
                            4,
                        );
                        brick[octant_flat_offset] = empty_marker::<PaletteIndexValues>();
                    }
                }
            }
        }
        assert!(
            data.is_empty_throughout(target_octant, 4, &color_palette, &data_palette),
            "Data cleared under octant should be empty"
        );
    }

    #[test]
    fn test_part_of_octant_empty() {
        // Create a filled parted Brick
        let color_palette = vec![Albedo::default().with_alpha(100); 1];
        let data_palette = vec![0u32; 1];
        let mut data =
            BrickData::<PaletteIndexValues>::Parted(vec![NodeContent::pix_visual(0); 4 * 4 * 4]);
        for i in 0..8 {
            for j in 0..8 {
                assert!(!data.is_part_empty_throughout(i, j, 4, &color_palette, &data_palette));
            }
        }

        let target_octant: u8 = 5;
        // offset by half of the brick dimension, as half of the dim 4x4x4 translates to 2x2x2
        // which is the resolution of OCTANT_OFFSET_REGION_LUT
        let octant_offset =
            V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[target_octant as usize] * 2.);

        // Erase part of the octant, the relevant part should be empty, while others should not be
        if let BrickData::Parted(ref mut brick) = data {
            let octant_flat_offset =
                flat_projection(octant_offset.x, octant_offset.y, octant_offset.z, 4);
            brick[octant_flat_offset] = empty_marker::<PaletteIndexValues>();
        }
        assert!(
            data.is_part_empty_throughout(target_octant, 0, 4, &color_palette, &data_palette),
            "Data cleared under part of octant should be empty"
        );
        assert!(
            !data.is_part_empty_throughout(target_octant, 1, 4, &color_palette, &data_palette),
            "Data not cleared should not be empty"
        );
    }
}

mod mipmap_tests {
    use crate::octree::{Albedo, Octree, V3c, OOB_OCTANT};

    #[test]
    fn test_mixed_mip_lvl1() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let mix: Albedo = 0x7F7F00FF.into();

        let mut tree: Octree = Octree::new(2, 1).ok().unwrap();
        tree.auto_simplify = false;
        tree.switch_albedo_mip_maps(true);
        tree.insert(&V3c::new(0, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 0, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 1, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 1, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(1, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(1, 0, 1), &green)
            .expect("octree insert");

        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            mix,
            *tree
                .sample_root_mip(OOB_OCTANT, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );
    }

    #[test]
    fn test_simple_solid_mip_lvl2_where_dim_is_2() {
        let red: Albedo = 0xFF0000FF.into();

        let mut tree: Octree = Octree::new(4, 2).ok().unwrap();
        tree.auto_simplify = false;
        tree.switch_albedo_mip_maps(true);
        tree.insert(&V3c::new(0, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 0, 1), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 1, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 1, 1), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(1, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(1, 0, 1), &red)
            .expect("octree insert");

        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            red,
            *tree
                .sample_root_mip(OOB_OCTANT, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(0, 0, 1))
            .albedo()
            .is_none());
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(0, 1, 0))
            .albedo()
            .is_none());
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(0, 1, 1))
            .albedo()
            .is_none());
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(1, 0, 0))
            .albedo()
            .is_none());
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(1, 0, 1))
            .albedo()
            .is_none());
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(1, 1, 0))
            .albedo()
            .is_none());
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(1, 1, 1))
            .albedo()
            .is_none());
    }

    #[test]
    fn test_mixed_mip_lvl2_where_dim_is_2() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let mix: Albedo = 0x7F7F00FF.into();

        let mut tree: Octree = Octree::new(4, 2).ok().unwrap();
        tree.auto_simplify = false;
        tree.switch_albedo_mip_maps(true);
        tree.insert(&V3c::new(0, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 0, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 1, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 1, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(1, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(1, 0, 1), &green)
            .expect("octree insert");

        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            mix,
            *tree
                .sample_root_mip(OOB_OCTANT, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(0, 0, 1))
            .albedo()
            .is_none());
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(0, 1, 0))
            .albedo()
            .is_none());
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(0, 1, 1))
            .albedo()
            .is_none());
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(1, 0, 0))
            .albedo()
            .is_none());
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(1, 0, 1))
            .albedo()
            .is_none());
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(1, 1, 0))
            .albedo()
            .is_none());
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(1, 1, 1))
            .albedo()
            .is_none());
    }

    #[test]
    fn test_mixed_mip_lvl2_where_dim_is_4() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let blue: Albedo = 0x0000FFFF.into();

        let mut tree: Octree = Octree::new(16, 4).ok().unwrap();
        tree.auto_simplify = false;
        tree.switch_albedo_mip_maps(true);
        tree.insert(&V3c::new(0, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 0, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 1, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 1, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(1, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(1, 0, 1), &green)
            .expect("octree insert");

        tree.insert(&V3c::new(8, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(8, 0, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(8, 1, 0), &blue)
            .expect("octree insert");
        tree.insert(&V3c::new(8, 1, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(9, 1, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(9, 0, 1), &blue)
            .expect("octree insert");

        // For child position 0,0,0
        let rg_mix: Albedo = 0x7F7F00FF.into();
        assert!(tree
            .sample_root_mip(0, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rg_mix,
            *tree
                .sample_root_mip(0, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );

        // For child position 8,0,0
        let rgb_mix: Albedo = 0x555555FF.into();
        assert!(tree
            .sample_root_mip(1, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rgb_mix,
            *tree
                .sample_root_mip(1, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );

        // root mip position 0,0,0
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rg_mix,
            *tree
                .sample_root_mip(OOB_OCTANT, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );

        // root mip position 8,0,0
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(2, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rgb_mix,
            *tree
                .sample_root_mip(OOB_OCTANT, &V3c::new(2, 0, 0))
                .albedo()
                .unwrap()
        );
    }

    #[test]
    fn test_mixed_mip_regeneration_lvl2_where_dim_is_4() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let blue: Albedo = 0x0000FFFF.into();

        let mut tree: Octree = Octree::new(16, 4).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(0, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 0, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 1, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(0, 1, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(1, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(1, 0, 1), &green)
            .expect("octree insert");

        tree.insert(&V3c::new(8, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(8, 0, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(8, 1, 0), &blue)
            .expect("octree insert");
        tree.insert(&V3c::new(8, 1, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(9, 1, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(9, 0, 1), &blue)
            .expect("octree insert");

        // Switch MIP maps on, it should calculate the correct values
        tree.switch_albedo_mip_maps(true);

        // For child position 0,0,0
        let rg_mix: Albedo = 0x7F7F00FF.into();
        assert!(tree
            .sample_root_mip(0, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rg_mix,
            *tree
                .sample_root_mip(0, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );

        // For child position 8,0,0
        let rgb_mix: Albedo = 0x555555FF.into();
        assert!(tree
            .sample_root_mip(1, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rgb_mix,
            *tree
                .sample_root_mip(1, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );

        // root mip position 0,0,0
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rg_mix,
            *tree
                .sample_root_mip(OOB_OCTANT, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );

        // root mip position 8,0,0
        assert!(tree
            .sample_root_mip(OOB_OCTANT, &V3c::new(2, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rgb_mix,
            *tree
                .sample_root_mip(OOB_OCTANT, &V3c::new(2, 0, 0))
                .albedo()
                .unwrap()
        );
    }
}
