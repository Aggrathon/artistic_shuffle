use std::collections::HashMap;
use std::path::PathBuf;
use id3::Tag;
use rand::prelude::SliceRandom;
use rand::seq::index::sample;

pub struct FileTree {
    tree : HashMap<String, Vec<PathBuf>>
}

impl FileTree {
    pub fn new() -> FileTree {
        FileTree { tree: HashMap::new() }
    }
    pub fn add_file(&mut self, file: PathBuf) {
        match Tag::read_from_path(&file) {
            Err(_) => {},
            Ok(tag) => {
                self.add(file, String::from(tag.artist().unwrap_or("")))
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

fn main() {
    println!("Hello, world!");
}
