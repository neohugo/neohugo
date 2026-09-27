//! Port of golang.org/x/text/collate/sort.go.

use super::Lister;

#[allow(dead_code)]
const MAX_SORT_BUFFER: usize = 40960;
#[allow(dead_code)]
const MAX_SORT_ENTRIES: usize = 4096;

/// Go `sorter`: keys are owned vectors instead of slices into a shared
/// `Buffer` (same bytes).
#[derive(Default)]
pub(crate) struct Sorter {
    pub keys: Vec<Vec<u8>>,
}

struct SortData<'a, L: Lister + ?Sized> {
    keys: &'a mut [Vec<u8>],
    src: &'a mut L,
}

impl<L: Lister + ?Sized> go_sort::sort::Interface for SortData<'_, L> {
    // Go: collate/sort.go:sorter.Len
    fn len(&self) -> usize {
        self.keys.len()
    }

    // Go: collate/sort.go:sorter.Less
    fn less(&mut self, i: usize, j: usize) -> bool {
        self.keys[i] < self.keys[j]
    }

    // Go: collate/sort.go:sorter.Swap
    fn swap(&mut self, i: usize, j: usize) {
        self.keys.swap(i, j);
        self.src.swap(i, j);
    }
}

impl Sorter {
    // Go: collate/sort.go:sorter.init
    pub(crate) fn init(&mut self, n: usize) {
        self.keys.clear();
        self.keys.resize(n, Vec::new());
    }

    // Go: collate/sort.go:sorter.sort
    pub(crate) fn sort<L: Lister + ?Sized>(&mut self, src: &mut L) {
        let mut d = SortData {
            keys: &mut self.keys,
            src,
        };
        go_sort::sort::sort(&mut d);
    }
}
