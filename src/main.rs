use std::collections::HashMap;
use std::path::{PathBuf, Component};
use id3::Tag;
use rand::prelude::SliceRandom;
use rand::seq::index::sample;
use path_abs::PathAbs;

pub struct FileMap {
    tree : HashMap<String, Vec<PathBuf>>
}

impl FileMap {
    pub fn new() -> FileMap {
        FileMap { tree: HashMap::new() }
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
    pub fn add(&mut self, file: PathBuf, band: String) {
        match self.tree.get_mut(&band) {
            Some(list) => { list.push(file); },
            None => { self.tree.insert(band, vec!(file)); }
        }
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

fn main() {
    println!("Hello, world!");
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_band() {
        let mut files = FileMap::new();
        files.add_file(PathBuf::from("a/b"));
        dbg!(&files.tree);
        assert!(files.tree.contains_key(&String::from("a")));
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
}
