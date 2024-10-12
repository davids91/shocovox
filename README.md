# Shocovox - Shady Octree Of Voxels with Ray Marching
Shocovox is a Sparse Voxel Octree implementation in GPU Shader Language(s) ( hence: shady ). 
The leaf nodes contain Voxel bricks instead of Voxels. This makes it possible to have a unique compression system, where Voxels of different resolutions can be mixed together.
A basic implementation for raytracing is available with GPU support!
The library uses Left handed Y up coordinate system.

Roadmap:
-
- Implementing Caching to request data on demand to handle large data: https://github.com/davids91/shocovox/milestone/3
- Displaying large data as a panorama: https://github.com/davids91/shocovox/milestone/1
- Finalising "Octree of Voxel Bricks" concept: https://github.com/davids91/shocovox/milestone/2

Issue spotlight: 
-
These are the issues I will work on until 2025 Q1. I am eliminating them in a sequential manner.
- #29 - Homogeneous nodes - an optimization that also enables a more efficient byte structure for the octrees
- #45 - GPU cache - To make it possible to display octrees of limitless size on the GPU by streaming only what one can see
- #3 - to make it possible to have a limitless octree: so it's not bound by the RAM size anymore
- #28, #6 - Level of Detail implementation to render large scenes more efficiently

Special thanks to contributors and supporters!
-

@nerdachse For the Albedo type and amazing support!

@DouglasDwyer My nemesis; Check out [his project](https://github.com/DouglasDwyer/octo-release) it's amazing! ( I hate him )

@Neo-Zhixing For [his amazing project](https://github.com/dust-engine) and awesome idea about how to utilize hardware RT for Voxel rendering!
