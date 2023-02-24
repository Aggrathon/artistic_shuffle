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

    pub fn iter(&self) -> std::collections::hash_map::Iter<T, usize> {
        self.0.iter()
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
        let mut prng = rand::thread_rng();
        self.order.shuffle(&mut prng);
        let n = self.order.len();
        let lookahead = std::cmp::min(max_lookahead, n / self.max_same);
        // SAFETY: The size of the list is guaranteed by modulo
        unsafe {
            let mut sweep = |i: usize| {
                let curr = *self.order.get_unchecked(i);
                let mut swp = i;
                let mut hit = false;
                for j in (i + 1)..(i + lookahead) {
                    let j = j % n;
                    let mut nex = *self.order.get_unchecked(j);
                    swp += 1;
                    while curr == nex {
                        hit = true;
                        swp += 1;
                        self.order.swap(j, swp % n);
                        nex = *self.order.get_unchecked(j);
                    }
                }
                swp = if hit { swp - i } else { 0 };
                swp
            };
            let mut chain: usize = 0;
            for i in 0..n {
                chain = std::cmp::max(chain.saturating_sub(1), sweep(i));
            }
            'outer: for _ in 0..5 {
                for i in 0..n {
                    if chain < 1 {
                        break 'outer;
                    }
                    chain = std::cmp::max(chain - 1, sweep(i));
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

    pub fn nested_iter(&self) -> TaggedShuffleNestedIterator<T> {
        TaggedShuffleNestedIterator {
            shuffle: self,
            outer: 0,
            inner: std::iter::repeat(0).take(self.items.len()).collect(),
        }
    }

    pub fn nested_add(&mut self, item: TaggedShuffle<T>) {
        let len = item.len();
        self.addn(item, len);
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
        // SAFETY: self.inner.len() always matches self.shuffle.items.len()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let mut cnt = Counter::new();
        cnt.add(0);
        cnt.add(2);
        cnt.addn(5, 10);
        cnt.addn(0, 2);
        let mut iter: Vec<_> = cnt.iter().map(|(a, b)| (*a, *b)).collect();
        iter.sort();
        assert_eq!(iter, vec![(0, 3), (2, 1), (5, 10)]);
    }

    #[test]
    fn test_shuffle() {
        for _ in 0..10 {
            let mut ts = TaggedShuffle::new();
            for i in 0..4 {
                ts.addn(i, i + 1);
            }
            ts.shuffle(10);
            assert!(ts.iter().zip(ts.iter().skip(1)).all(|(a, b)| a != b));
        }
    }

    #[test]
    fn test_nested_shuffle() {
        let mut ts = TaggedShuffle::new();
        for i in 0..4 {
            let mut ts2 = TaggedShuffle::new();
            ts2.addn(i, 4);
            ts.nested_add(ts2);
        }
        ts.nested_shuffle(10);
        // dbg!(ts.nested_iter().collect::<Vec<_>>());
        assert!(ts
            .iter()
            .zip(ts.iter().skip(1))
            .all(|(a, b)| !std::ptr::eq(a, b)));
        assert!(ts
            .nested_iter()
            .zip(ts.nested_iter().skip(1))
            .all(|(a, b)| a != b));
    }
}
