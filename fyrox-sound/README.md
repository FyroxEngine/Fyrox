# fyrox-sound

Sound library for games and interactive applications written in Rust.

**NOTE:** even though this crate has `fyrox` prefix in its name, it can be used separately without any issues.

## Key features

- Generic and Spatial sound sources.
- Built-in streaming for large sounds.
- Raw samples playback support.
- WAV format support (non-compressed).
- Vorbis/ogg support (using [lewton](https://crates.io/crates/lewton)).
- [HRTF](https://en.wikipedia.org/wiki/Head-related_transfer_function) support for excellent positioning and binaural effects.
- Reverb effect.

## Examples

Examples can be found in `./examples`. Make sure you run examples with `--release` flag, `debug` version is very slow and may cause tearing of output sound.

## Supported OS

- Windows (DirectSound)
- Linux (alsa)
- macOS (CoreAudio)
- WebAssembly (WebAudio)
- Android (AAudio, API Level 26+)

## HRTF

This library has full HRTF support, it uses HRIR spheres generated using [IRCAM](http://recherche.ircam.fr/equipes/salles/listen/) HRIR database. HRIR spheres are produced using a small tool written in C++ - [hrir_sphere_builder](https://github.com/mrDIMAS/hrir_sphere_builder ). It is very important to find HRTF that suits you because they're very individual and the overall perception is fully defined by the use of correct HRTF.

## Contributions

Any contributions are very appreciated! Check the `Issues` page to see how can you help the project. 

## License

MIT

## References

This library wouldn't have been ever created without work of these people. Thank you all!

1. [Digital signal processing and filters](https://ccrma.stanford.edu/~jos/filters/) 
2. [Physical Audio Signal Processing](https://ccrma.stanford.edu/~jos/pasp/)
3. Hannes Gamper, "Head-related transfer function interpolation in azimuth, elevation, and distance", The Journal of the Acoustical Society of America 134, EL547 (2013); doi: 10.1121/1.4828983
4. Fábio P. Freeland, Luiz W. P. Biscainho, Paulo S. R. Diniz, "Interpolation of Head-related transfer function (HRTFS): A Multi-source approarch"
5. [IRCAM Head-related impulse response database](http://recherche.ircam.fr/equipes/salles/listen/)
6. [Reverb](https://ccrma.stanford.edu/~jos/pasp/Freeverb.html)
7. [Overlap-add convolution](https://en.wikipedia.org/wiki/Overlap%E2%80%93add_method) - not used anymore due to significant distortions at segment boundary when impulse response changes.
8. [Overlap-save convolution](https://dsp-nbsphinx.readthedocs.io/en/nbsphinx-experiment/nonrecursive_filters/segmented_convolution.html) - works much better when impulse response changes, there are only phase shift issues which are more or less acceptable.
9. [OpenAL Specification](https://www.openal.org/documentation/openal-1.1-specification.pdf) - distance models and general design considerations.
10. http://csoundjournal.com/issue9/newHRTFOpcodes.html - some ideas to remove clicks in hrtf renderer
11. https://phaidra.kug.ac.at/open/o:11024
