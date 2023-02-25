use pathdiff::diff_paths;
use shuffle::{Counter, Shuffler};
use std::collections::HashMap;
use std::env::args;
use std::fs::{create_dir_all, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

mod shuffle;
mod tags;

pub struct Playlist(HashMap<String, Counter<PathBuf>>);

impl Playlist {
    pub fn new() -> Playlist {
        Playlist(HashMap::new())
    }

    // Add a song with known band and rating
    pub fn add(&mut self, file: PathBuf, band: String, rating: Option<u8>) {
        let band = band.trim().to_lowercase();
        // A rating of "200" is "4/5"
        let times = rating.map(|r| r / 200 + 1).unwrap_or(1) as usize;
        match self.0.get_mut(&band) {
            Some(counter) => counter.addn(file, times),
            None => {
                let mut counter = Counter::new();
                counter.addn(file, times);
                self.0.insert(band, counter);
            }
        }
    }

    /// Add the path to the playlist (recursively if it is a directory)
    pub fn add_path(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.add_dir(path)
        } else {
            self.add_file(path)
        }
    }

    pub fn add_file(&mut self, file: PathBuf) {
        let (band, rating) = tags::get_tags(&file);
        self.add(file, band, rating);
    }

    /// Add a file with a different output path
    pub fn add_file2(&mut self, file: &Path, path: PathBuf) {
        let (band, rating) = tags::get_tags(file);
        self.add(path, band, rating);
    }

    fn add_dir(&mut self, path: PathBuf) {
        let is_not_hidden = |e: &DirEntry| {
            e.file_name()
                .to_str()
                .map(|s| !s.starts_with('.'))
                .unwrap_or(true)
        };
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_entry(is_not_hidden)
        {
            match entry {
                Ok(entry) => self.add_file(entry.path().to_path_buf()),
                Err(error) => eprintln!("Could not access file: {}", error),
            }
        }
    }

    /// Read the contents of the file and add to the playlist (recursively if it is a directory)
    pub fn read_path(&mut self, path: PathBuf) {
        match path.metadata() {
            Ok(md) => {
                if md.is_dir() {
                    self.read_dir(path)
                } else if md.is_file() {
                    self.read_file(&path)
                } else {
                    eprintln!("Unknown type of object: {}", path.to_string_lossy())
                }
            }
            Err(e) => eprintln!("Error accessing path '{}': {}", path.to_string_lossy(), e),
        }
    }

    /// Read and add files from a file (e.g. playlist)
    fn read_file(&mut self, file: &Path) {
        let parent = file.parent();
        if let Ok(f) = File::open(file) {
            for line in BufReader::new(f).lines().flatten() {
                let path = PathBuf::from(line);
                if parent.is_none() || path.is_absolute() {
                    self.add_file(path);
                } else {
                    #[allow(clippy::unnecessary_unwrap)]
                    self.add_file2(&parent.unwrap().join(&path), path);
                }
            }
        }
    }

    fn read_dir(&mut self, path: PathBuf) {
        let is_not_hidden = |e: &DirEntry| {
            e.file_name()
                .to_str()
                .map(|s| !s.starts_with('.'))
                .unwrap_or(true)
        };
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_entry(is_not_hidden)
        {
            match entry {
                Ok(entry) => self.read_file(entry.path()),
                Err(error) => eprintln!("Could not access file: {}", error),
            }
        }
    }

    // Create a list of all songs in the filemap with an artist-aware shuffle
    pub fn shuffle(&self) -> Shuffler<Shuffler<&PathBuf>> {
        let mut ts = shuffle::Shuffler::new();
        for (_, counter) in self.0.iter() {
            let mut ts2 = shuffle::Shuffler::new();
            for (p, n) in counter.iter() {
                ts2.addn(p, *n);
            }
            ts.nested_add(ts2);
        }
        ts.nested_shuffle(10);
        ts
    }
}

impl Default for Playlist {
    fn default() -> Self {
        Self::new()
    }
}

// Get the path of the parent dir (handling also relative paths)
fn get_parent_dir(path: &Path) -> PathBuf {
    if path.is_relative() {
        path.parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
    } else {
        path.parent()
            .unwrap_or_else(|| path.ancestors().last().unwrap())
            .to_path_buf()
    }
}

// States for parsing commandline parameters
#[derive(PartialEq, Eq)]
enum State {
    Init,
    Input,
    Middle,
    Output,
}

// The main function parses commandline parameters, reads files, shuffles, and outputs playlist(s)
fn main() {
    let mut files = Playlist::new();
    let mut state: State = State::Init;
    let mut iter = args();
    iter.next();
    for a in iter {
        let a = a.trim();
        match state {
            // Handling adding of files to the filemap
            State::Init | State::Input => {
                if a == "--" {
                    state = State::Middle;
                    if state == State::Init {
                        files.read_dir(PathBuf::from("."));
                    }
                } else {
                    let path = PathBuf::from(a);
                    if path.exists() {
                        if path.is_dir() {
                            files.add_dir(path);
                        } else if path.is_file() {
                            files.read_file(&path);
                        }
                    } else {
                        eprintln!("Input path {} doesn't exist", path.to_string_lossy());
                    }
                    state = State::Input;
                }
            }
            // Handling shuffling and outputting
            State::Middle | State::Output => {
                state = State::Output;
                let path = PathBuf::from(a);
                let parent = get_parent_dir(&path);
                create_dir_all(&parent).unwrap_or_default();
                match File::create(&path) {
                    Err(e) => println!("{}", e),
                    Ok(mut file) => {
                        for f in files.shuffle().nested_iter() {
                            if f.is_absolute() {
                                writeln!(file, "{}", f.to_string_lossy())
                                    .expect("Could not write to file");
                            } else {
                                match diff_paths(f, &parent) {
                                    Some(f) => writeln!(file, "{}", f.to_string_lossy())
                                        .expect("Could not write to file"),
                                    None => writeln!(file, "{}", f.to_string_lossy())
                                        .expect("Could not write to file"),
                                };
                            };
                        }
                    }
                }
            }
        }
    }
    // Checking the ending state for insertion of default behaviour (help etc.)
    match state {
        State::Middle => {
            for f in files.shuffle().nested_iter() {
                println!("{}", f.to_string_lossy());
            }
        }
        State::Input | State::Init => {
            help();
        }
        _ => {}
    }
}

// Print usage help
fn help() {
    let exe = args()
        .next()
        .unwrap_or_else(|| String::from("cargo run --"));
    println!("Description:");
    println!("  Create a shuffled playlist where songs from the same artist are spread out");
    println!("  The artist names are taken from the files' ID3-tags.");
    println!("  If a tag is missing then the artist is based on the filename (first directory not in the base path).");
    println!("  The output paths will be global/local depending on the input paths.");
    println!("\nUsage:\n  {} INPUTS -- OUTPUTS", &exe);
    println!("\nArguments:");
    println!("  INPUTS   are directories or .m3u/.csv/.txt files (\".\" if empty)");
    println!("  OUTPUTS  are files (output to terminal if empty)");
    println!("\nExamples:\n  {} ~/Music -- playlist.m3u\n  {} playlist1.m3u playlist2.m3u -- shuffled.m3u", &exe, &exe);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let mut pl = Playlist::new();
        pl.add_file(PathBuf::from("a/b"));
        pl.add_file2(&PathBuf::from("d/e/f"), PathBuf::from("d"));
        pl.add_dir(PathBuf::from("src"));
        assert!(pl.0.contains_key(&String::from("a")));
        assert!(pl.0.contains_key(&String::from("d")));
    }

    #[test]
    fn test_shuffle() {
        let mut pl = Playlist::new();
        pl.add(PathBuf::from("a"), String::from("a"), None);
        pl.add(PathBuf::from("b"), String::from("a"), Some(199));
        pl.add(PathBuf::from("c"), String::from("b"), Some(200));
        pl.add(PathBuf::from("d"), String::from("b"), Some(201));

        let shuff = pl.shuffle().nested_iter().copied().collect::<Vec<_>>();

        assert!(shuff.contains(&&PathBuf::from("a")));
        assert!(shuff.contains(&&PathBuf::from("b")));
        assert!(shuff.contains(&&PathBuf::from("c")));
        assert!(shuff.contains(&&PathBuf::from("d")));
        assert_eq!(shuff.len(), 6);
    }
}
