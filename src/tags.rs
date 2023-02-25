use lofty::{read_from_path, ItemKey, Tag, TagType, TaggedFileExt};
use std::path::{Component, Path};

pub fn get_tags(path: &Path) -> (String, Option<u8>) {
    let mut artist = None;
    let mut rating = None;
    if let Ok(tagged_file) = read_from_path(path) {
        let mut artist2 = None;
        if let Some(tag) = tagged_file.primary_tag() {
            artist2 = parse_artist(tag);
            rating = parse_rating(tag);
        }
        for tag in tagged_file.tags() {
            if artist2.is_some() && rating.is_some() {
                break;
            }
            if artist2.is_none() {
                artist2 = parse_artist(tag);
            }
            if rating.is_none() {
                rating = parse_rating(tag);
            }
        }
        artist = artist2.map(String::from);
    }
    (
        artist.unwrap_or_else(|| parse_artist_from_path(path)),
        rating,
    )
}

fn parse_artist(tag: &Tag) -> Option<&str> {
    tag.get_string(&ItemKey::TrackArtist)
        .or_else(|| tag.get_string(&ItemKey::AlbumArtist))
        .or_else(|| tag.get_string(&ItemKey::OriginalArtist))
        .or_else(|| tag.get_string(&ItemKey::Performer))
        .or_else(|| tag.get_string(&ItemKey::Composer))
}

fn parse_rating(tag: &Tag) -> Option<u8> {
    tag.get_item_ref(&ItemKey::Popularimeter)?;
    match tag.tag_type() {
        TagType::APE => todo!(),
        TagType::ID3v1 => todo!(),
        TagType::ID3v2 => parse_rating_binaryu8(tag),
        TagType::MP4ilst => parse_rating_text100(tag),
        TagType::VorbisComments => parse_rating_text100(tag),
        TagType::RIFFInfo => todo!(),
        TagType::AIFFText => todo!(),
        _ => todo!(),
    }
}

fn parse_rating_binaryu8(tag: &Tag) -> Option<u8> {
    let bin = tag.get_binary(&ItemKey::Popularimeter, false)?;
    if bin.len() > 5 {
        Some(bin[bin.len() - 5])
    } else {
        None
    }
}

fn parse_rating_text100(tag: &Tag) -> Option<u8> {
    let s = tag.get_string(&ItemKey::Popularimeter)?;
    let v = s.parse::<u8>().ok()?;
    Some(v * 2 + v / 2)
}

// Parse a path to try to guess the artist name
fn parse_artist_from_path(path: &Path) -> String {
    if let Some(parent) = path.parent() {
        let (p1, p2) = parent.components().fold((None, None), |p, c| {
            if let Component::Normal(name) = c {
                (Some(name), p.0)
            } else {
                p
            }
        });
        if let Some(p) = p2 {
            return p.to_string_lossy().to_string();
        }
        if let Some(p) = p1 {
            return p.to_string_lossy().to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_path_band() {
        let path = PathBuf::from("a");
        assert!(parse_artist_from_path(&path).is_empty());
        let path = path.join("b");
        assert_eq!(parse_artist_from_path(&path), "a");
        let path = path.join("c");
        assert_eq!(parse_artist_from_path(&path), "a");
        let path = path.join("d");
        assert_eq!(parse_artist_from_path(&path), "b");
    }

    #[test]
    #[ignore = "Audio metadata is tricky, this is a realworld test that is not meant to be run regularly."]
    fn parse_your_music() {
        let path = dirs::audio_dir().unwrap();
        for entry in walkdir::WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .flatten()
        {
            if let Ok(tags) = read_from_path(entry.path()) {
                for tag in tags.tags() {
                    if let Err(e) = std::panic::catch_unwind(|| {
                        parse_artist(tag);
                        assert!(!(2..15).contains(&parse_rating(tag).unwrap_or(100)));
                    }) {
                        dbg!(tag.items().collect::<Vec<_>>());
                        dbg!(tag.tag_type());
                        dbg!(entry.path());
                        panic!("{:?}", e);
                    }
                }
            }
        }
    }
}
