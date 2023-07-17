
struct V3c{
	x: f32, y: f32, z:f32
}

struct Node<Content> {
    min_position: V3c,
    size: f32,
    content: Content,
    //TODO: children? def not recursively, but maybe an index of its child
}

use crate::object_pool::ObjectPool;
struct Octree <Content>{
	nodes: ObjectPool<Node<Content>>,
}
