// TODO:
// - add Vulkan API wrapper/bevy wrapper for raycasting( or both? )
// - octants to have names after all?
// - Implement lazy-loading
// - make ObjectPool threadSafe independenct of octree
// - make Octree Thread-safe with an RwLock implementation

pub mod spatial;
pub mod octree;
pub mod object_pool;