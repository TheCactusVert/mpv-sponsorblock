# SponsorBlock plugin for MPV
A port of [SponsorBlock](https://github.com/ajayyy/SponsorBlock) for MPV (or Celluloid) written in Rust.

- Why write a MPV plugin in Rust ?
Why not!

- Can I write my own plugin in Rust ?
Yes! Just follow the example [here](https://crates.io/crates/mpv-client) and you will be ready.

## Rquirements
### Build dependencies
- rust
- openssl
### Runtime dependencies
- openssl

## Build
Build the plugin:
```bash
rustup override set nightly
cargo build --release
```

## Installation
### Plugin
- **MPV**: copy the lib generated to your `scripts` folder:
```bash
cp ./target/release/libmpv_sponsorblock.so ~/.config/mpv/scripts/sponsorblock.so
```
- **Celluloid** : copy the lib generated to your `scripts` folder:
```bash
cp ./target/release/libmpv_sponsorblock.so ~/.config/celluloid/scripts/sponsorblock.so
```

## Configuration
Copy the exemple configuration file `sponsorblock.toml` into your **MPV** folder:
```bash
cp ./sponsorblock.toml ~/.config/mpv/sponsorblock.toml
```

If no configuration file is found, only the sponsors segments will be skipped as specified by the [API](https://wiki.sponsor.ajay.app/w/API_Docs).

## Keybindings

### Highlight
You can add a binding to jump to the highlight of the video by adding this line to `input.conf`:
```
alt+p script-binding "sponsorblock/poi"
```

You also need to add these values to their associated keys in your `sponsorblock.toml`:
```toml
categories = ["poi_highlight"]
action_types = ["poi"]
```

## Usage
Play a YouTube video and segments you chose in the configuration file will be skipped or muted. If the video is entirely labeled as a category it will be shown at startup :
![celluloid](images/celluloid.png)
