# rg3d-sound

Sound library for games written in Rust.

## Key features

- Spatial and flat sounds
- Built-in streaming for large sounds
- WAV format support (non-compressed)
- [HRTF](https://en.wikipedia.org/wiki/Head-related_transfer_function) support for excellent positioning and binaural effects.

## Examples

Examples can be found in `./examples`

Make sure you run examples with `--release` flag, `debug` version is very slow and may cause tearing of output sound.

## Supported OS

- Windows (DirectSound)
- Linux (alsa)

## How to build

Add `rg3d-sound = "0.3.0"` to your Cargo.toml

## HRTF

Library has full HRTF support, it uses HRIR spheres generated using [IRCAM](http://recherche.ircam.fr/equipes/salles/listen/) HRIR database. HRIR spheres are produced using small tool written in C++ - [hrir_sphere_builder](https://github.com/mrDIMAS/hrir_sphere_builder )

## Help needed

It would be great if someone make backend for macOS.

## License

MIT