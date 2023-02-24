use rand::seq::SliceRandom;
use std::collections::HashMap;

pub struct Counter<T: std::hash::Hash + std::cmp::Eq>(HashMap<T, usize>);

pub struct TaggedShuffle<T> {
    items: Vec<T>,
    order: Vec<usize>,
    max_same: usize,
}

pub struct TaggedShuffleIterator<'a, T> {
    shuffle: &'a TaggedShuffle<T>,
    index: usize,
}
pub struct TaggedShuffleNestedIterator<'a, T> {
    shuffle: &'a TaggedShuffle<TaggedShuffle<T>>,
    outer: usize,
    inner: Vec<usize>,
}

impl<T: std::hash::Hash + std::cmp::Eq> Counter<T> {
    pub fn new() -> Counter<T> {
        Counter(HashMap::new())
    }

    pub fn add(&mut self, item: T) {
        self.addn(item, 1);
    }

    pub fn addn(&mut self, item: T, num: usize) {
        let cnt = match self.0.get(&item).copied() {
            Some(cnt) => cnt + num,
            None => num,
        };
        self.0.insert(item, cnt);
    }
}

impl<T: std::hash::Hash + std::cmp::Eq> Default for Counter<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TaggedShuffle<T> {
    pub fn new() -> TaggedShuffle<T> {
        TaggedShuffle {
            items: Vec::new(),
            order: Vec::new(),
            max_same: 1,
        }
    }

    pub fn add(&mut self, item: T) {
        self.order.push(self.items.len());
        self.items.push(item);
    }

    pub fn addn(&mut self, item: T, num: usize) {
        self.max_same = std::cmp::max(self.max_same, num);
        let i = self.items.len();
        self.order.extend(std::iter::repeat(i).take(num));
        self.items.push(item);
    }

    pub fn shuffle(&mut self, max_lookahead: usize) {
        if self.order.is_empty() {
            return;
        }
        self.order.shuffle(&mut rand::thread_rng());
        let n = self.order.len();
        let lookahead = std::cmp::min(max_lookahead, n / self.max_same);
        // SAFETY: The size of the list is guaranteed by modulo
        unsafe {
            let mut sweep = |i: usize| {
                let curr = *self.order.get_unchecked(i);
                let after = i + lookahead;
                let mut swp = 0;
                for j in (i + 1)..(i + lookahead) {
                    let j = j % n;
                    let mut nex = *self.order.get_unchecked(j);
                    while curr == nex {
                        swp += 1;
                        self.order.swap(j, (swp + after) % n);
                        nex = *self.order.get_unchecked(j);
                    }
                }
                swp
            };
            let mut chain = 0;
            for i in 0..n {
                chain = std::cmp::max(chain, sweep(i));
            }
            let mut count = 0;
            for i in 0..(n - lookahead) {
                let swp = sweep(i);
                if swp == 0 {
                    count += 1;
                    if count >= chain {
                        break;
                    }
                } else {
                    chain = std::cmp::max(chain, swp);
                    count = 0;
                }
            }
        }
    }

    /// # Safety
    /// This is safe if index < self.len()
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        // SAFETY: self.order can only contain indices from self.item
        self.items.get_unchecked(*self.order.get_unchecked(index))
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        // SAFETY: self.order can only contain indices from self.item
        unsafe { Some(self.items.get_unchecked(*self.order.get(index)?)) }
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn iter(&self) -> TaggedShuffleIterator<T> {
        TaggedShuffleIterator {
            shuffle: self,
            index: 0,
        }
    }
}

impl<T> TaggedShuffle<TaggedShuffle<T>> {
    pub fn nested_shuffle(&mut self, max_lookahead: usize) {
        for rnd in self.items.iter_mut() {
            rnd.shuffle(max_lookahead);
        }
        self.shuffle(max_lookahead);
    }

    pub fn nested_iter(&mut self) -> TaggedShuffleNestedIterator<T> {
        TaggedShuffleNestedIterator {
            shuffle: self,
            outer: 0,
            inner: std::iter::repeat(0).take(self.items.len()).collect(),
        }
    }
}

impl<T> Default for TaggedShuffle<T>
where
    T: std::cmp::Eq,
    T: std::hash::Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> Iterator for TaggedShuffleIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let out = self.shuffle.get(self.index);
        self.index += 1;
        out
    }
}

impl<'a, T> Iterator for TaggedShuffleNestedIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let i = *self.shuffle.order.get(self.outer)?;
        let j;
        // SAFETY: self.inner.len() always matches self.shuffle.items_len()
        unsafe {
            let inner = self.inner.get_unchecked_mut(i);
            j = *inner;
            *inner += 1;
        }
        let out = self.shuffle.get(self.outer)?.get(j);
        self.outer += 1;
        out
    }
}
