mod mipmap_tests {
    use crate::octree::{Albedo, BoxTree, MIPResamplingMethods, V3c, OOB_SECTANT};

    #[test]
    fn test_mixed_mip_lvl1() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let mix: Albedo = (
            // Gamma corrected values follow mip = ((a^2 + b^2) / 2).sqrt()
            (((255_f32.powf(2.) / 2.).sqrt() as u32) << 16)
                | (((255_f32.powf(2.) / 2.).sqrt() as u32) << 24)
                | 0x000000FF
        )
        .into();

        let mut tree: BoxTree = BoxTree::new(4, 1).ok().unwrap();
        tree.auto_simplify = false;
        tree.albedo_mip_map_resampling_strategy()
            .switch_albedo_mip_maps(true)
            .set_method_at(1, MIPResamplingMethods::BoxFilter);
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
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            mix,
            *tree
                .albedo_mip_map_resampling_strategy()
                .sample_root_mip(OOB_SECTANT, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );
    }

    #[test]
    fn test_mixed_mip_lvl1_where_dim_is_32() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let mix: Albedo = (
            // Gamma corrected values follow mip = ((a^2 + b^2) / 2).sqrt()
            (((255_f32.powf(2.) / 2.).sqrt() as u32) << 16)
                | (((255_f32.powf(2.) / 2.).sqrt() as u32) << 24)
                | 0x000000FF
        )
        .into();

        let mut tree: BoxTree = BoxTree::new(128, 32).ok().unwrap();
        tree.auto_simplify = false;
        tree.albedo_mip_map_resampling_strategy()
            .switch_albedo_mip_maps(true)
            .set_method_at(1, MIPResamplingMethods::BoxFilter);
        tree.insert(&V3c::new(126, 126, 126), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(126, 126, 127), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(126, 127, 126), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(126, 127, 127), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(127, 126, 126), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(127, 126, 127), &green)
            .expect("octree insert");

        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(31, 31, 31))
            .albedo()
            .is_some());
        assert_eq!(
            mix,
            *tree
                .albedo_mip_map_resampling_strategy()
                .sample_root_mip(OOB_SECTANT, &V3c::new(31, 31, 31))
                .albedo()
                .unwrap()
        );
    }

    #[test]
    fn test_simple_solid_mip_lvl2_where_dim_is_2() {
        let red: Albedo = 0xFF0000FF.into();

        let mut tree: BoxTree = BoxTree::new(8, 2).ok().unwrap();
        tree.auto_simplify = false;
        tree.albedo_mip_map_resampling_strategy()
            .switch_albedo_mip_maps(true)
            .set_method_at(1, MIPResamplingMethods::BoxFilter);
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
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            red,
            *tree
                .albedo_mip_map_resampling_strategy()
                .sample_root_mip(OOB_SECTANT, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(0, 0, 1))
            .albedo()
            .is_none());
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(0, 1, 0))
            .albedo()
            .is_none());
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(0, 1, 1))
            .albedo()
            .is_none());
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(1, 0, 0))
            .albedo()
            .is_none());
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(1, 0, 1))
            .albedo()
            .is_none());
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(1, 1, 0))
            .albedo()
            .is_none());
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(1, 1, 1))
            .albedo()
            .is_none());
    }

    #[test]
    fn test_mixed_mip_lvl2_where_dim_is_2() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let mix: Albedo = (
            // Gamma corrected values follow mip = ((a^2 + b^2) / 2).sqrt()
            (((255_f32.powf(2.) / 2.).sqrt() as u32) << 16)
                | (((255_f32.powf(2.) / 2.).sqrt() as u32) << 24)
                | 0x000000FF
        )
        .into();

        let mut tree: BoxTree = BoxTree::new(8, 2).ok().unwrap();
        tree.auto_simplify = false;
        tree.albedo_mip_map_resampling_strategy()
            .switch_albedo_mip_maps(true)
            .set_method_at(1, MIPResamplingMethods::BoxFilter);
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
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            mix,
            *tree
                .albedo_mip_map_resampling_strategy()
                .sample_root_mip(OOB_SECTANT, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(0, 0, 1))
            .albedo()
            .is_none());
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(0, 1, 0))
            .albedo()
            .is_none());
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(0, 1, 1))
            .albedo()
            .is_none());
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(1, 0, 0))
            .albedo()
            .is_none());
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(1, 0, 1))
            .albedo()
            .is_none());
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(1, 1, 0))
            .albedo()
            .is_none());
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(1, 1, 1))
            .albedo()
            .is_none());
    }

    #[test]
    fn test_mixed_mip_lvl2_where_dim_is_4() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let blue: Albedo = 0x0000FFFF.into();

        let mut tree: BoxTree = BoxTree::new(64, 4).ok().unwrap();
        tree.auto_simplify = false;
        tree.albedo_mip_map_resampling_strategy()
            .switch_albedo_mip_maps(true)
            .set_method_at(1, MIPResamplingMethods::BoxFilter)
            .set_method_at(2, MIPResamplingMethods::BoxFilter);
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

        tree.insert(&V3c::new(16, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(16, 0, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(16, 1, 0), &blue)
            .expect("octree insert");
        tree.insert(&V3c::new(16, 1, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(17, 1, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(17, 0, 1), &blue)
            .expect("octree insert");

        // For child position 0,0,0
        let rg_mix: Albedo = (
            // Gamma corrected values follow mip = ((a^2 + b^2) / 2).sqrt()
            (((255_f32.powf(2.) / 2.).sqrt() as u32) << 16)
                | (((255_f32.powf(2.) / 2.).sqrt() as u32) << 24)
                | 0x000000FF
        )
        .into();
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(0, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rg_mix,
            *tree
                .albedo_mip_map_resampling_strategy()
                .sample_root_mip(0, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );

        // For child position 16,0,0
        let rgb_mix: Albedo = (
            // Gamma corrected values follow mip = ((a^2 + b^2) / 2).sqrt()
            (((255_f32.powf(2.) / 3.).sqrt() as u32) << 8)
                | (((255_f32.powf(2.) / 3.).sqrt() as u32) << 16)
                | (((255_f32.powf(2.) / 3.).sqrt() as u32) << 24)
                | 0x000000FF
        )
        .into();
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(1, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rgb_mix,
            *tree
                .albedo_mip_map_resampling_strategy()
                .sample_root_mip(1, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );

        // root mip position 0,0,0
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rg_mix,
            *tree
                .albedo_mip_map_resampling_strategy()
                .sample_root_mip(OOB_SECTANT, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );

        // root mip position 16,0,0
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(1, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rgb_mix,
            *tree
                .albedo_mip_map_resampling_strategy()
                .sample_root_mip(OOB_SECTANT, &V3c::new(1, 0, 0))
                .albedo()
                .unwrap()
        );
    }

    #[test]
    fn test_mixed_mip_regeneration_lvl2_where_dim_is_4() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let blue: Albedo = 0x0000FFFF.into();

        let mut tree: BoxTree = BoxTree::new(64, 4).ok().unwrap();
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

        tree.insert(&V3c::new(16, 0, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(16, 0, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(16, 1, 0), &blue)
            .expect("octree insert");
        tree.insert(&V3c::new(16, 1, 1), &green)
            .expect("octree insert");
        tree.insert(&V3c::new(17, 1, 0), &red)
            .expect("octree insert");
        tree.insert(&V3c::new(17, 0, 1), &blue)
            .expect("octree insert");

        // Switch MIP maps on, calculate the correct values
        tree.albedo_mip_map_resampling_strategy()
            .switch_albedo_mip_maps(true)
            .set_method_at(1, MIPResamplingMethods::BoxFilter)
            .set_method_at(2, MIPResamplingMethods::BoxFilter)
            .recalculate_mips();

        // For child position 0,0,0
        let rg_mix: Albedo = (
            // Gamma corrected values follow mip = ((a^2 + b^2) / 2).sqrt()
            (((255_f32.powf(2.) / 2.).sqrt() as u32) << 16)
                | (((255_f32.powf(2.) / 2.).sqrt() as u32) << 24)
                | 0x000000FF
        )
        .into();
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(0, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rg_mix,
            *tree
                .albedo_mip_map_resampling_strategy()
                .sample_root_mip(0, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );

        // For child position 8,0,0
        let rgb_mix: Albedo = (
            // Gamma corrected values follow mip = ((a^2 + b^2) / 2).sqrt()
            (((255_f32.powf(2.) / 3.).sqrt() as u32) << 8)
                | (((255_f32.powf(2.) / 3.).sqrt() as u32) << 16)
                | (((255_f32.powf(2.) / 3.).sqrt() as u32) << 24)
                | 0x000000FF
        )
        .into();
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(1, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rgb_mix,
            *tree
                .albedo_mip_map_resampling_strategy()
                .sample_root_mip(1, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );

        // root mip position 0,0,0
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(0, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rg_mix,
            *tree
                .albedo_mip_map_resampling_strategy()
                .sample_root_mip(OOB_SECTANT, &V3c::new(0, 0, 0))
                .albedo()
                .unwrap()
        );

        // root mip position 16,0,0
        assert!(tree
            .albedo_mip_map_resampling_strategy()
            .sample_root_mip(OOB_SECTANT, &V3c::new(1, 0, 0))
            .albedo()
            .is_some());
        assert_eq!(
            rgb_mix,
            *tree
                .albedo_mip_map_resampling_strategy()
                .sample_root_mip(OOB_SECTANT, &V3c::new(1, 0, 0))
                .albedo()
                .unwrap()
        );
    }
}
