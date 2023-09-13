// TODO:
// - add Vulkan API wrapper/bevy wrapper for raycasting( or both? )
// - Implement internal representations: best approximation of leaf nodes in intermediate Nodes
// - Not all intermediate nodes might have a simplified representation!
// --> set a threshold for the percentage/count of cubes need be identical to be able to build a simplified parent representation
// - Implement lazy-loading
// - make ObjectPool threadSafe independenct of octree
// - make Octree Thread-safe with an RwLock implementation
// - Get by ray at LOD
// - Root node is always at index 0. simplify Octree!
// - Re-check values in case there are more, than u32::MAX nodes
// - FOV is actually inverseFOV.. the bigger the value of that, the more focused the viewport is
// - Octree::get_by_ray could return a distance value instead of the impact point
// - Current Octree size limiatation because of the shader is 32768; Evaluate/increase size  and document limiations!

pub mod spatial;
pub mod octree;
pub mod object_pool;