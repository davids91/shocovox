// TODO:
// - create trait for data instead of the generic T; T would also implement Bencode traits so user doesn't have to
// - add Vulkan API wrapper/bevy wrapper for raycasting( or both? )
// - octants to have names after all?
// - have a safeguard for tests where infinite loop is expected
// - Implement lazy-loading
// - make ObjectPool threadSafe independenct of octree
// - make Octree Thread-safe with an RwLock implementation
// - sort out includes ( uses )
// - get_step_to_next_sibling should never return with 0 length vector

pub mod spatial;
pub mod octree;
pub mod object_pool;