# Artistic Shuffle

When shuffling a playlist the goal is seldom *randomness*, but rather *variety*.
This program aims to provide an alternative shuffle where no artist is repeated too often.
Furthermore, favourite tracks (4/5 ★ and up) occur twice as often as the other tracks.

If the files are accessible, metadata is used for the artist names and ratings.
If the metadata is missing, the artist is based on the path (assuming an `artist/album/track` directory stucture).

## Usage

```artistic_shuffle INPUT -r READ -o OUTPUT```

| Argument | Description | Note |
|---------:|-------------|------|
| INPUT    | Files to add to the playlist (directories are recursively added). | Accepts multiple |
| READ     | Read a list of files to add to the playlist (from files such as `.m3u`/`.csv`/`.txt`) | Accepts multiple |
| OUTPUT   | Where to write the shuffled playlist (outputs to the terminal if missing). | Accepts multiple |

This tool will preserve relative paths.

## Examples

`artistic_shuffle --help`  
`artistic_shuffle ~/Music -o playlist.m3u`  
`artistic_shuffle favourite_song.mp3 -r playlist1.m3u -r playlist2.m3u -o shuffled.m3u`

## Building

1. Install Rust
2. Download this repo
3. Run `cargo build --release`
4. The executable can be found in `target/release`

Alternatively you can find some prebuilt binaries in [releases](https://github.com/Aggrathon/artistic_shuffle/releases).
