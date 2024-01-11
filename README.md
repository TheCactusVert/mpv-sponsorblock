<h1 align="center">SponsorBlock plugin for MPV</h1>

A port of [SponsorBlock](https://github.com/ajayyy/SponsorBlock) for MPV (or Celluloid) written in Rust.

## Questions

### Can I write my own plugin in Rust ?

Yes! Just follow the example [here](https://crates.io/crates/mpv-client) and you will be ready.

## Build

Build the plugin:

```bash
cargo build --release --locked
```

## Installation

Installation is not available for Windows : https://mpv.io/manual/stable/#c-plugins

<details>
<summary>MPV</summary>

Copy the lib generated to your `scripts` folder:

```bash
cp ./target/release/libmpv_sponsorblock.so ~/.config/mpv/scripts/sponsorblock.so
```

</details>

<details>
<summary>Celluloid</summary>

Copy the lib generated to your `scripts` folder:

```bash
cp ./target/release/libmpv_sponsorblock.so ~/.config/celluloid/scripts/sponsorblock.so
```

</details>

## Configuration

Copy the exemple configuration file `sponsorblock.toml` into your **MPV** (not Celluloid) folder:
```bash
cp ./sponsorblock.toml ~/.config/mpv/sponsorblock.toml
```

If no configuration file is found, only the sponsors segments will be skipped as specified by the [API](https://wiki.sponsor.ajay.app/w/API_Docs).

A segment is the combination of a category and an action type.

### Categories

Official SponsorBlock documentation on categories can be found [here](https://wiki.sponsor.ajay.app/w/Guidelines#Category_Breakdown).

Here is a summary :
- `sponsor`: Part of a video promoting a product or service not directly related to the creator.
- `selfpromo`: Promoting a product or service that is directly related to the creator themselves.
- `interaction`: Explicit reminders to like, subscribe or interact with them on any paid or free platform(s) (e.g. click on a video).
- `poi_highlight`: Used to get to the point or highlight of a video.
- `intro`: Segments typically found at the start of a video that include an animation, still frame or clip which are also seen in other videos by the same creator.
- `outro`: Segments typically near or at the end of the video when the credits pop up and/or endcards are shown.
- `preview`: Collection of clips that show what is coming up in in this video or other videos in a series where all information is repeated later in the video.
- `music_offtopic`: Non-music Section segments on videos which feature music as the primary content.
- `filler`: Filler Tangent/ Jokes is only for tangential scenes added only for filler or humor that are not required to understand the main content of the video.
- `exclusive_access`: When the creator showcases a product, service or location that they've received free or subsidised access to in the video that cannot be completely removed by cuts.

### Action Types

Here is a summary :
- `skip`: The segment will be skipped by the plugin.
- `mute`: The segment will be muted by the plugin. These segments are not skipped! See the documentation [here](https://wiki.sponsor.ajay.app/w/Mute_Segment).
- `full`: The video is marked as a category you requested. The plugin will only show a message at the start of the video. See the documentation [here](https://wiki.sponsor.ajay.app/w/Full_Video_Labels).
- `poi`: Allow you to jump to the highlight of the video. Do not forget to add the associated keybinding and the category `poi_highlight`. See the documentation [here](https://wiki.sponsor.ajay.app/w/Highlight).

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

Play a YouTube video and segments you chose in the configuration file will be skipped or muted.

If the video is entirely labeled as a category it will be shown at startup :
![celluloid](images/celluloid.png)
