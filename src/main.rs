use std::collections::HashMap;
use std::path::{PathBuf, Component};
use std::env::args;
use id3::Tag;
use rand::prelude::SliceRandom;
use rand::seq::index::sample;
use pathdiff::diff_paths;

pub struct FileMap {
    tree : HashMap<String, Vec<PathBuf>>
}

impl FileMap {
    pub fn new() -> FileMap {
        FileMap { tree: HashMap::new() }
    }
    pub fn add(&mut self, file: PathBuf, band: String) {
        match self.tree.get_mut(&band) {
            Some(list) => { list.push(file); },
            None => { self.tree.insert(band, vec!(file)); }
        }
    }
    pub fn add_file(&mut self, file: PathBuf) {
        match Tag::read_from_path(&file) {
            Err(_) => {
                let band = root_named_dir(&file);
                self.add(file, band)
            },
            Ok(tag) => {
                let band = if let Some(name) = tag.artist() {
                    String::from(name)
                } else {
                    root_named_dir(&file)
                };
                self.add(file, band)
            }
        }
    }
    pub fn add_relative(&mut self, file: PathBuf, path: &PathBuf) {
        let band = match Tag::read_from_path(&file) {
            Err(_) => {
                match diff_paths(&file, &path) {
                    Some(p) => root_named_dir(&p),
                    None => String::from("")
                }
            },
            Ok(tag) => {
                match tag.artist() {
                    Some(name) => String::from(name),
                    None => match diff_paths(&file, &path) {
                        Some(p) => root_named_dir(&p),
                        None => String::from("")
                    }
                }
            }
        };
        self.add(file, band)
    }
    pub fn read_dir(&mut self, dir: PathBuf) {
        if !dir.exists() || !dir.is_dir() { return; }
        let mut stack: Vec<PathBuf> = vec!();
        stack.push(PathBuf::from(&dir));
        while let Some(d) = stack.pop() {
            match d.read_dir() {
                Err(e) => println!("{}", e),
                Ok(iter) => {
                    iter.for_each(|entry| {
                        match entry {
                            Err(e) => println!("{}", e),
                            Ok(item) => {
                                if !item.file_name().to_string_lossy().starts_with(".") {
                                    let p: PathBuf = item.path();
                                    if p.is_dir() {
                                        stack.push(p);
                                    } else {
                                        self.add_relative(p, &dir);
                                    }
                                }
                            }
                        }
                    })
                }
            }
        };
    }
    pub fn shuffle(&self) -> Vec<&PathBuf> {
        let mut list : Vec<&PathBuf> = vec!();
        if self.tree.is_empty() { return list; }
        let mut len : usize = 0;
        let mut rng = rand::thread_rng();
        self.tree.iter().for_each(|(_, v)| {
            if len < v.len() { len = v.len(); }
            let start = list.len();
            list.extend(v.iter());
            list[start..].shuffle(&mut rng);
        });
        if list.is_empty() { return list; }
        let mut list : Vec<&PathBuf> = sample(&mut rng, len, len).into_iter().flat_map(|i| {
            list.iter().skip(i).step_by(len)
        }).map(|p| *p).collect();
        let stride : usize = (list.len()-1)/len/2+1;
        list.chunks_mut(stride).for_each(|c| c.shuffle(&mut rng));
        list
    }
}

fn root_named_dir(path: &PathBuf) -> String {
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

#[derive(PartialEq,Eq)]
enum State {
    Init,
    Input,
    Middle,
    Output
}

fn main() {
    let mut files = FileMap::new();
    let mut state: State = State::Init;
    let mut iter = args();
    iter.next();
    while let Some(a) = iter.next() {
        match state {
            State::Init | State::Input => {
                if a == "--" {
                    state = State::Middle;
                    if state == State::Init { files.read_dir(PathBuf::from(".")); }
                } else {
                    files.read_dir(PathBuf::from(a));
                    //TODO: Read from file
                    state = State::Input;
                }
            },
            State::Middle | State::Output => {
                state = State::Output;
                println!("TODO: write to file");
            }
        }
    }
    match state {
        State::Middle => {
            for f in files.shuffle() {
                println!("{}", f.to_string_lossy());
            }
        },
        State::Input | State::Init => {
            help();
        },
        _ => {}
    }
}

fn help() {
    let exe = args().next().unwrap_or(String::from("cargo run"));
    println!("Description:\n  TODO: Description");
    println!("\nUsage:\n  {} INPUTS -- OUTPUTS", &exe);
    println!("\nArguments:");
    println!("  INPUTS   are directiories or .m3u/.csv/.txt files (\".\" if empty)");
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

        assert!(shuff[..2].contains(&&PathBuf::from("a")) || shuff[..2].contains(&&PathBuf::from("b")));
        assert!(shuff[..2].contains(&&PathBuf::from("c")) || shuff[..2].contains(&&PathBuf::from("d")));
    }


    #[test]
    fn test_dir() {
        let mut files = FileMap::new();
        files.read_dir(PathBuf::from("."));
    }
}
