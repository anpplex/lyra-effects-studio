# Lyra Effects Studio

Lyra Effects Studio is a native macOS editor, previewer, device debugger, and signed Theme Registry toolchain for [Lyra](https://github.com/anpplex/Lyra) lyric effects.

The project is in active development. Its first milestone establishes the public Pack and Registry contracts before the visual editor and Android bridge are added.

## Requirements

- macOS 14 or later
- Xcode 26 or a compatible Swift 6.2 toolchain

## Build

```sh
swift test
swift build --product LyraEffectsStudio
swift run lyra-effects --version
```

## License

The application and SDK are licensed under Apache-2.0. Theme Packs retain their own licenses and notices.
