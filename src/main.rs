use id3::Tag;
use pathdiff::diff_paths;
use rand::prelude::SliceRandom;
use rand::seq::index::sample;
use std::collections::HashMap;
use std::env::args;
use std::fs::{create_dir_all, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};

// Datastructure for holding the list of files, based on a hashmap
pub struct FileMap {
    tree: HashMap<String, Vec<PathBuf>>,
}

impl FileMap {

    // Constructor that initialises the map
    pub fn new() -> FileMap {
        FileMap {
            tree: HashMap::new(),
        }
    }

    // Add a song with known band
    pub fn add(&mut self, file: PathBuf, band: String) {
        let band = band.trim().to_lowercase();
        match self.tree.get_mut(&band) {
            Some(list) => {
                list.push(file);
            }
            None => {
                self.tree.insert(band, vec![file]);
            }
        }
    }
    pub fn add_file(&mut self, file: PathBuf) {
        // Add a song based on a path
        let band = get_artist(&file);
        self.add(file, band);
    }

    // Add a song based on a relative path
    pub fn add_relative(&mut self, file: PathBuf, base: &PathBuf) {
        let band = get_artist_relative(&file, &base);
        self.add(file, band);
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
                        if !item.file_name().to_string_lossy().starts_with(".") {
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
            for l in BufReader::new(f).lines() {
                if let Ok(line) = l {
                    let path = PathBuf::from(line);
                    if path.is_absolute() && file.is_absolute() {
                        self.add_relative(path, &parent);
                    } else {
                        self.add(parent.join(&path), get_artist(&path));
                    }
                }
            }
        }
    }

    // Create a list of all songs in the filemap, and so a artist-aware shuffle
    pub fn shuffle(&self) -> Vec<&PathBuf> {
        let mut list: Vec<&PathBuf> = vec![];
        if self.tree.is_empty() {
            return list;
        }
        let mut len: usize = 0;
        let mut rng = rand::thread_rng();
        // Add all songs from each band (shuffled artisti-wise), and find the maximum number of songs for one band
        self.tree.iter().for_each(|(_, v)| {
            if len < v.len() {
                len = v.len();
            }
            let start = list.len();
            list.extend(v.iter());
            list[start..].shuffle(&mut rng);
        });
        if list.is_empty() {
            return list;
        }
        // Reorder the list so that the songs from the same band are spread out
        let mut list: Vec<&PathBuf> = sample(&mut rng, len, len)
            .into_iter()
            .flat_map(|i| list.iter().skip(i).step_by(len))
            .map(|p| *p)
            .collect();
        // Do some local randomisation (to not always have the same order of artists)
        let stride: usize = (list.len() * 2) / (len * 5) + 1;
        list.chunks_mut(stride).for_each(|c| c.shuffle(&mut rng));
        list
    }
}

// Get the artist from a file (first try reading the meta data then parsing the path)
fn get_artist(file: &PathBuf) -> String {
    match Tag::read_from_path(&file) {
        Err(_) => get_artist_from_path(&file),
        Ok(tag) => match tag.artist() {
            Some(name) => String::from(name),
            None => get_artist_from_path(&file),
        },
    }
}

// Get the artist from a relative file (first try reading the meta data then parsing the relative path)
fn get_artist_relative(file: &PathBuf, base: &PathBuf) -> String {
    if let Ok(tag) = Tag::read_from_path(&file) {
        if let Some(name) = tag.artist() {
            return String::from(name);
        }
    };
    match diff_paths(&file, &base) {
        Some(p) => get_artist_from_path(&p),
        None => String::from(""),
    }
}

// Parse a path to try to guess the band name
fn get_artist_from_path(path: &PathBuf) -> String {
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
fn get_parent_dir(path: &PathBuf) -> PathBuf {
    if path.is_relative() {
        path.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        path.parent()
            .unwrap_or(path.ancestors().last().unwrap())
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
    let mut files = FileMap::new();
    let mut state: State = State::Init;
    let mut iter = args();
    iter.next();
    while let Some(a) = iter.next() {
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
                        for f in files.shuffle() {
                            if f.is_absolute() {
                                write!(file, "{}\n", f.to_string_lossy())
                                    .expect("Could not write to file");
                            } else {
                                match diff_paths(&f, &parent) {
                                    Some(f) => write!(file, "{}\n", f.to_string_lossy())
                                        .expect("Could not write to file"),
                                    None => write!(file, "{}\n", f.to_string_lossy())
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
            for f in files.shuffle() {
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
    let exe = args().next().unwrap_or(String::from("cargo run --"));
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
        let mut files = FileMap::new();
        files.add_file(PathBuf::from("a/b"));
        files.add_relative(PathBuf::from("d/e/f"), &PathBuf::from("d"));
        dbg!(&files.tree);
        assert!(files.tree.contains_key(&String::from("a")));
        assert!(files.tree.contains_key(&String::from("e")));
    }

    #[test]
    fn test_shuffle() {
        let mut files = FileMap::new();
        files.add(PathBuf::from("a"), String::from("a"));
        files.add(PathBuf::from("b"), String::from("a"));
        files.add(PathBuf::from("c"), String::from("b"));
        files.add(PathBuf::from("d"), String::from("b"));

        let shuff = files.shuffle();

        assert!(shuff.contains(&&PathBuf::from("a")));
        assert!(shuff.contains(&&PathBuf::from("b")));
        assert!(shuff.contains(&&PathBuf::from("c")));
        assert!(shuff.contains(&&PathBuf::from("d")));

        assert!(
            shuff[..2].contains(&&PathBuf::from("a")) || shuff[..2].contains(&&PathBuf::from("b"))
        );
        assert!(
            shuff[..2].contains(&&PathBuf::from("c")) || shuff[..2].contains(&&PathBuf::from("d"))
        );
    }

    #[test]
    fn test_dir() {
        let mut files = FileMap::new();
        files.read_dir(PathBuf::from("."));
    }
}
