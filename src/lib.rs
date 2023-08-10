// TODO:
// - switch from exit distance logic to flip logic 
// --> exit points matching coordinate with the nodes center flips the idx on the corresponding coordinate
// - consider if storing midpoint in the cube would worth it; or if min_position and size neds to be stored
// - create trait for data instead of the generic T
// - add Vulkan API wrapper/bevy wrapper ( or both? )
// - octants to have names after all?
// - Remove debug traits

pub mod spatial;
pub mod octree;
pub mod object_pool;