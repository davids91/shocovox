// TODO:
// - create trait for data instead of the generic T
// - add example for presentation purpose
// - add Vulkan API wrapper
// - consider if storing midpoint in the cude would worth it
// - consider if min_position and size is even something that needs to be stored on the GPU
// - test bounds_at and child_bounds_for functions
// - test ray marching if the ray starts from inside the octree
// - octants to have names after all? 
// - reduce intersections to 1 per loop in octree::get_by_ray
// - update raycast logic so exit point can be used without ambiguity and thresholds?


pub mod spatial;
pub mod octree;
pub mod object_pool;