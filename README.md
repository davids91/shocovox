# Shocovox - Shady Octree Of Voxels with Ray Marching
Shocovox is a Sparse Voxel Octree implementation in WGPU shaders ( hence: shady ).
The leaf nodes contain 8 Voxel bricks instead of a single Voxel. This makes it possible to have a unique compression system, where Voxels of different resolutions can be mixed together.
An implementation for raytracing is available with GPU support!
The library uses Left handed Y up coordinate system.

Roadmap:
-
- Implementing Caching to request data on demand to handle large data: https://github.com/davids91/shocovox/milestone/3
- Displaying large data as a panorama: https://github.com/davids91/shocovox/milestone/1
- Finalising "Octree of Voxel Bricks" concept: https://github.com/davids91/shocovox/milestone/2

Issue spotlight: 
-
These are the issues I will work on until 2025 Q2. I am eliminating them in a sequential manner.
- #56 - Introduce Palettes - Trimming the fat in Voxel storage, broadening the possibilities with user data and eliminating some data conversion overhead with bricks.
- #65 - Flatten brick storage: trimming some additional overhead, and eliminating some possible techDebt (`DIM` generic argument with Octree)
- #3 - to make it possible to have a limitless octree: so it's not bound by the RAM size anymore
- #17 Beam Optimization - Pre-render a small resolution image to optimally initialize ray distances, and help with deciding which bricks to load pre-emptively. GOTTA GO FAST
- #28, #6 - Level of Detail implementation to render large scenes more efficiently

If you feel adventurous:
-

I have marked some issues with the help needed flag, which I think would be a good addition to the library, but I can not focus on it as I am but a single person with limited time and resources. Feel free to try to tackle any of the marked issues (Or basically anything you'd like), I will provide any needed help and support if I can. 

Special thanks to contributors and supporters!
-

[@nerdachse](https://github.com/nerdachse) For the Albedo type and amazing support!

[@DouglasDwyer](https://github.com/DouglasDwyer) My nemesis; Check out [his project](https://github.com/DouglasDwyer/octo-release) it's amazing! ( I hate him )

[@Neo-Zhixing](https://github.com/Neo-Zhixing) For [his amazing project](https://github.com/dust-engine) and awesome idea about how to utilize hardware RT for Voxel rendering!
