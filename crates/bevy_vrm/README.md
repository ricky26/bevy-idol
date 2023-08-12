A crate implementing support for the VRM specification for Bevy.

This crate only aims to target VRMC specifications post-1.0.

Elements supported so far:
- [ ] VRM file loading
  - [x] Basic GLTF parsing
  - [x] VRM metadata parsing (VRMC_vrm-1.0)
  - [x] Humanoid bone parsing
  - [x] Look-at parsing
  - [x] Apply look-at at runtime
- [ ] MToon Shading (VRM_materials_mtoon-1.0)
  - [x] Parsing mtoon structures from VRM
  - [x] Base color shading
  - [ ] Outline shading
- [ ] Spring bones (VRMC_springBone-1.0)
- [ ] Node constraints (VRMC_node_constraint-1.0)
- [ ] Animations (VRMC_vrm_animation-1.0)
