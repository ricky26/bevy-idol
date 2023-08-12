VTuber software written in Rust on Bevy.

bevy-idol renders a VRM avatar to a transparent window, designed to be used as
an overlay in OBS.

## Running Bevy Idol
- Clone & run [mediapipe-vtube](https://github.com/ricky26/mediapipe-vtube)
- Run with run `cargo run --bin=bevy_idol -- -W 768 -H 1080`
  - (The specified width & height control the overlay window.)
- You can add HANATool blend shapes automatically by specifying the path with 
  `--extra-blend-shapes=HANA_Tool/BlendShapeData/PerfectSync_VRoid_v1_0_0_Female.txt`.
- The preview window will show the scene with the inspector, and a second transparent output window will draw the 
  final result.

## Crates
### bevy_idol
This is the main application which will load a VRM and render it to a
transparent window.

### bevy_vrm
A crate implementing support for the VRM specifications.

### idol_api
A crate containing the API types which bevy-idol uses to communicate with external software. (This will probably be 
removed at some point.)

## Why?
I don't really have a good answer for this one. I liked the idea of getting something like this to work, but a lot 
of existing solutions required loading up quite a few heavy pieces of software to get a good result - and I love 
working in Rust with Bevy. It seemed like a good little challenge.
