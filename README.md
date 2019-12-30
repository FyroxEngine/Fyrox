# rg3d-sound

Sound library for games written in Rust with HRTF support.

## Key features

- Spatial and flat sounds
- Built-in streaming for large sounds
- WAV format support (non-compressed)
- Vorbis/ogg support (using [lewton](https://crates.io/crates/lewton))
- [HRTF](https://en.wikipedia.org/wiki/Head-related_transfer_function) support for excellent positioning and binaural effects.
- Reberb effect

## Examples

Examples can be found in `./examples`

Make sure you run examples with `--release` flag, `debug` version is very slow and may cause tearing of output sound.

## Supported OS

- Windows (DirectSound)
- Linux (alsa)

## How to build

Add `rg3d-sound = "0.5.0"` to your Cargo.toml
Supported Rust version is >= 1.38

## HRTF

Library has full HRTF support, it uses HRIR spheres generated using [IRCAM](http://recherche.ircam.fr/equipes/salles/listen/) HRIR database. HRIR spheres are produced using small tool written in C++ - [hrir_sphere_builder](https://github.com/mrDIMAS/hrir_sphere_builder )

## Help needed

It would be great if someone make backend for macOS.

## License

MIT

## References

1. [Digital signal processing and filters](https://ccrma.stanford.edu/~jos/filters/) 
2. [Physical Audio Signal Processing](https://ccrma.stanford.edu/~jos/pasp/)
3. Hannes Gamper, "Head-related transfer function interpolation in azimuth, elevation, and distance", The Journal of the Acoustical Society of America 134, EL547 (2013); doi: 10.1121/1.4828983
4. Fábio P. Freeland, Luiz W. P. Biscainho, Paulo S. R. Diniz, "Interpolation of Head-related transfer function (HRTFS): A Multi-source approarch"
5. [IRCAM Head-related impulse response database](http://recherche.ircam.fr/equipes/salles/listen/)
6. [Reverb](https://ccrma.stanford.edu/~jos/pasp/Freeverb.html)
7. [Overlap-add convolution](https://en.wikipedia.org/wiki/Overlap%E2%80%93add_method)