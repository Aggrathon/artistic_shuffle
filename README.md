# Artistic Shuffle

When shuffling a playlist the goal is seldom *randomness*, but rather *variety*.
This program aims to provide an alternative shuffle where clusters with songs from the same artist are avoided (and vice versa a common artist should show up regularly throughout the playlist).

The artist names are taken from the *artist* ID3-tag.
If the tag is missing then the artist is based on the filename (first directory not in the base path).

## Usage

`artistic_shuffle INPUTS -- OUTPUTS`

|Arguments:||
|---|---|
|INPUTS  | are directories or .m3u/.csv/.txt files ("." if empty) |
|OUTPUTS | are files (output to terminal if empty) |

The output paths will be global/local depending on the input paths.


## Examples

`artistic_shuffle ~/Music -- playlist.m3u`  
`artistic_shuffle playlist1.m3u playlist2.m3u -- shuffled.m3u`