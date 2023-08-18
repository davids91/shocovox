// TODO:
// - create trait for data instead of the generic T; T would also implement Bencode traits so user doesn't have to
// - Actually implement raycast logic
// - add Vulkan API wrapper/bevy wrapper for raycasting( or both? )
// - octants to have names after all?
// - Remove debug traits
// - have a safeguard for tests where infinite loop is expected
// - Implement lazy-loading

pub mod spatial;
pub mod octree;
pub mod object_pool;