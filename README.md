# Shocovox - Shady Octree Of Voxels with Ray Marching
Shocovox is a Sparse Voxel Octree implementation in WGPU shaders ( hence: shady ).
The leaf nodes contain 8 Voxel bricks instead of a single Voxel. This makes it possible to have a unique compression system, where Voxels of different resolutions can be mixed together.
An implementation for raytracing is available with GPU support!
The library uses Left handed Y up coordinate system.

Archived in favor of: https://github.com/Ministry-of-Voxel-Affairs/VoxelHex/
