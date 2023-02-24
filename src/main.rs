use id3::{Tag, TagLike};
use pathdiff::diff_paths;
use shuffle::{Counter, Shuffler};
use std::collections::HashMap;
use std::env::args;
use std::fs::{create_dir_all, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};

mod shuffle;

// Datastructure for holding the list of files, based on a hashmap
pub struct Playlist(HashMap<String, Counter<PathBuf>>);

impl Playlist {
    // Constructor that initialises the map
    pub fn new() -> Playlist {
        Playlist(HashMap::new())
    }

    // Add a song with known band
    pub fn add(&mut self, file: PathBuf, band: String, times: usize) {
        let band = band.trim().to_lowercase();
        match self.0.get_mut(&band) {
            Some(counter) => counter.addn(file, times),
            None => {
                let mut counter = Counter::new();
                counter.addn(file, times);
                self.0.insert(band, counter);
            }
        }
    }

    pub fn add_file(&mut self, file: PathBuf) {
        // Add a song based on a path
        let band = get_artist(&file);
        self.add(file, band, 1);
    }

    // Add a song based on a relative path
    pub fn add_relative(&mut self, file: PathBuf, base: &Path) {
        let band = get_artist_relative(&file, base);
        self.add(file, band, 1);
    }

    // Read and add files from a directory
    pub fn read_dir(&mut self, dir: PathBuf) {
        if !dir.exists() || !dir.is_dir() {
            return;
        }
        let mut stack: Vec<PathBuf> = vec![];
        stack.push(PathBuf::from(&dir));
        while let Some(d) = stack.pop() {
            if let Ok(iter) = d.read_dir() {
                iter.for_each(|entry| {
                    if let Ok(item) = entry {
                        if !item.file_name().to_string_lossy().starts_with('.') {
                            let p: PathBuf = item.path();
                            if p.is_dir() {
                                stack.push(p);
                            } else {
                                self.add_relative(p, &dir);
                            }
                        }
                    }
                })
            }
        }
    }

    // Read and add files from a file (e.g. playlist)
    pub fn read_file(&mut self, file: PathBuf) {
        if !file.exists() || !file.is_file() {
            return;
        }
        let parent = get_parent_dir(&file);
        if let Ok(f) = File::open(&file) {
            for line in BufReader::new(f).lines().flatten() {
                let path = PathBuf::from(line);
                if path.is_absolute() && file.is_absolute() {
                    self.add_relative(path, &parent);
                } else {
                    self.add(parent.join(&path), get_artist(&path), 1);
                }
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

// Get the artist from a file (first try reading the meta data then parsing the path)
fn get_artist(file: &PathBuf) -> String {
    match Tag::read_from_path(file) {
        Err(_) => get_artist_from_path(file),
        Ok(tag) => match tag.artist() {
            Some(name) => String::from(name),
            None => get_artist_from_path(file),
        },
    }
}

// Get the artist from a relative file (first try reading the meta data then parsing the relative path)
fn get_artist_relative(file: &PathBuf, base: &Path) -> String {
    if let Ok(tag) = Tag::read_from_path(file) {
        if let Some(name) = tag.artist() {
            return String::from(name);
        }
    };
    match diff_paths(file, base) {
        Some(p) => get_artist_from_path(&p),
        None => String::from(""),
    }
}

// Parse a path to try to guess the band name
fn get_artist_from_path(path: &Path) -> String {
    if let Some(parent) = path.parent() {
        for dir in parent.components() {
            if let Component::Normal(name) = dir {
                if let Some(nstr) = name.to_str() {
                    return String::from(nstr);
                }
            }
        }
    }
    String::from("")
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
                            files.read_dir(path);
                        } else if path.is_file() {
                            files.read_file(path);
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
    fn test_path_band() {
        let mut files = Playlist::new();
        files.add_file(PathBuf::from("a/b"));
        files.add_relative(PathBuf::from("d/e/f"), &PathBuf::from("d"));
        // dbg!(&files.tree);
        assert!(files.0.contains_key(&String::from("a")));
        assert!(files.0.contains_key(&String::from("e")));
    }

    #[test]
    fn test_shuffle() {
        let mut files = Playlist::new();
        files.add(PathBuf::from("a"), String::from("a"), 1);
        files.add(PathBuf::from("b"), String::from("a"), 1);
        files.add(PathBuf::from("c"), String::from("b"), 1);
        files.add(PathBuf::from("d"), String::from("b"), 1);

        let shuff = files.shuffle().nested_iter().copied().collect::<Vec<_>>();

        assert!(shuff.contains(&&PathBuf::from("a")));
        assert!(shuff.contains(&&PathBuf::from("b")));
        assert!(shuff.contains(&&PathBuf::from("c")));
        assert!(shuff.contains(&&PathBuf::from("d")));
    }

    #[test]
    fn test_dir() {
        let mut files = Playlist::new();
        files.read_dir(PathBuf::from("."));
    }
}
