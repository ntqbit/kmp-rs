pub trait KmpSearchable {
    fn is_match_possible(&self, other: &Self) -> bool;

    fn is_match_guaranteed(&self, other: &Self) -> bool;
}

pub trait KmpMatchable<H> {
    fn match_haystack(&self, other: &H) -> bool;
}

trait KmpPrimitive: PartialEq {}

impl KmpPrimitive for u8 {}
impl KmpPrimitive for char {}
impl KmpPrimitive for bool {}

impl<T: KmpPrimitive> KmpSearchable for T {
    fn is_match_guaranteed(&self, other: &Self) -> bool {
        self == other
    }

    fn is_match_possible(&self, other: &Self) -> bool {
        self == other
    }
}

impl<T: KmpPrimitive> KmpMatchable<T> for T {
    fn match_haystack(&self, other: &T) -> bool {
        self == other
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KmpTableItem {
    needle: usize,
    haystack: usize,
}

pub type KmpTable<'a> = &'a [KmpTableItem];
pub type KmpOwnedTable = Vec<KmpTableItem>;

fn kmp_table<N: KmpSearchable>(needle: &[N]) -> KmpOwnedTable {
    if needle.is_empty() {
        return vec![];
    }

    let mut lsp: KmpOwnedTable = Vec::with_capacity(needle.len());
    lsp.push(KmpTableItem {
        needle: 0,
        haystack: 0,
    });

    for needle_item in &needle[1..] {
        let mut item = *lsp.last().unwrap();

        loop {
            if needle_item.is_match_possible(&needle[item.needle]) {
                if item.haystack == 0 {
                    if !needle_item.is_match_guaranteed(&needle[item.needle]) {
                        item.haystack = 1;
                    }
                } else {
                    item.haystack += 1;
                }

                item.needle += 1;
                break;
            }

            if item.needle == 0 {
                break;
            }

            item = lsp[item.needle - 1];
        }

        lsp.push(item);
    }

    lsp
}

pub struct KmpPattern<'a, N> {
    needle: &'a [N],
    lsp: KmpOwnedTable,
}

impl<'a, N> KmpPattern<'a, N> {
    pub fn new(needle: &'a [N]) -> Self
    where
        N: KmpSearchable,
    {
        let table = kmp_table(needle);

        Self { needle, lsp: table }
    }

    pub fn table(&self) -> KmpTable {
        &self.lsp
    }

    pub fn find<H>(&'a self, haystack: &'a [H]) -> KmpSearch<'a, N, H, false>
    where
        N: KmpMatchable<H>,
    {
        KmpSearch::new(self.needle, &self.lsp, haystack)
    }

    pub fn find_overlapping<H>(&'a self, haystack: &'a [H]) -> KmpSearch<'a, N, H, true>
    where
        N: KmpMatchable<H>,
    {
        KmpSearch::new(self.needle, &self.lsp, haystack)
    }
}

pub struct KmpSearch<'a, N, H, const OVERLAPPING: bool> {
    needle: &'a [N],
    lsp: &'a [KmpTableItem],
    haystack: &'a [H],
    needle_pos: usize,
    haystack_pos: usize,
}

impl<'a, N, H, const OVERLAPPING: bool> KmpSearch<'a, N, H, OVERLAPPING> {
    pub fn new(needle: &'a [N], lsp: &'a [KmpTableItem], haystack: &'a [H]) -> Self {
        Self {
            needle,
            lsp,
            haystack,
            needle_pos: 0,
            haystack_pos: 0,
        }
    }
}

impl<'a, N, H, const OVERLAPPING: bool> Iterator for KmpSearch<'a, N, H, OVERLAPPING>
where
    N: KmpMatchable<H>,
{
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let needle_len = self.needle.len();

        if self.haystack_pos + needle_len - self.needle_pos > self.haystack.len() {
            return None;
        }

        if needle_len == 0 {
            self.haystack_pos += 1;
            return Some(self.haystack_pos - 1);
        }

        loop {
            if self.haystack_pos >= self.haystack.len() {
                return None;
            }

            let mut haystack_item = &self.haystack[self.haystack_pos];
            self.haystack_pos += 1;

            loop {
                if self.needle[self.needle_pos].match_haystack(haystack_item) {
                    self.needle_pos += 1;

                    if self.needle_pos != needle_len {
                        break;
                    }

                    let match_pos = self.haystack_pos - needle_len;

                    if OVERLAPPING {
                        let back = self.lsp[self.needle_pos - 1];
                        self.needle_pos = back.needle;
                        if back.haystack != 0 {
                            self.needle_pos -= back.haystack;
                            self.haystack_pos -= back.haystack;
                        }
                    } else {
                        self.needle_pos = 0;
                    }

                    return Some(match_pos);
                }

                if self.needle_pos == 0 {
                    break;
                }

                let back = &self.lsp[self.needle_pos - 1];
                self.needle_pos = back.needle;
                if back.haystack != 0 {
                    self.needle_pos -= back.haystack;
                    self.haystack_pos -= back.haystack;
                    haystack_item = &self.haystack[self.haystack_pos];
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{KmpMatchable, KmpPattern, KmpSearchable};

    #[test]
    fn kmp_test() {
        const TEST_CASES: &[(&[u8], &[u8], &[usize])] = &[
            (b"abc", b"abc", &[0]),
            (b"abc", b"abcdef", &[0]),
            (b"def", b"abcdef", &[3]),
            (b"bcd", b"abcdef", &[1]),
            (b"xyz", b"abcdef", &[]),
            (b"", b"abcdef", &[0, 1, 2, 3, 4, 5, 6]),
            (b"abc", b"", &[]),
            (b"a", b"aaaaa", &[0, 1, 2, 3, 4]),
            (b"aa", b"aaaaa", &[0, 1, 2, 3]),
            (b"abcdef", b"abc", &[]),
            (b"a.b", b"a.b", &[0]),
            (b"aBc", b"AbCaBcD", &[3]),
            (b"aBc", b"AbCaBCd", &[]),
        ];

        for (needle, haystack, correct_matches) in TEST_CASES {
            let found_matches = kmp_match_overlapping(needle, haystack);
            assert_eq!(
                found_matches.as_slice(),
                *correct_matches,
                "needle: {:?}, haystack: {:?}",
                needle,
                haystack
            );
        }
    }

    fn kmp_match_overlapping<N, H>(needle: &[N], haystack: &[H]) -> Vec<usize>
    where
        N: KmpSearchable + KmpMatchable<H>,
    {
        let pattern = KmpPattern::new(needle);
        let found_matches: Vec<_> = pattern.find_overlapping(&haystack).collect();
        found_matches
    }

    // Tests taken from https://gitlab.com/bit-refined/kmp/
    mod matches {
        use crate::{KmpMatchable, KmpPattern, KmpSearchable};

        fn kmp_match<N, H>(needle: &[N], haystack: &[H]) -> Vec<usize>
        where
            N: KmpSearchable + KmpMatchable<H>,
        {
            let pattern = KmpPattern::new(needle);
            let found_matches: Vec<_> = pattern.find(&haystack).collect();
            found_matches
        }

        #[test]
        fn basic() {
            assert_eq!(
                vec![0, 6, 12],
                kmp_match(
                    &['a', 'b', 'c'],
                    &['a', 'b', 'c', 'X', 'X', 'X', 'a', 'b', 'c', 'Y', 'Y', 'Y', 'a', 'b', 'c'],
                )
            );
        }

        #[test]
        fn concatenated() {
            assert_eq!(
                vec![1, 4],
                kmp_match(&['a', 'b', 'c'], &['1', 'a', 'b', 'c', 'a', 'b', 'c', '2'])
            );
        }

        #[test]
        fn combined() {
            assert_eq!(
                vec![1],
                kmp_match(&['a', 'b', 'a'], &['1', 'a', 'b', 'a', 'b', 'a', '2'])
            );
        }

        #[test]
        fn empty_needle() {
            assert_eq!(vec![0, 1, 2], kmp_match::<char, _>(&[], &['a', 'b']));
        }

        #[test]
        fn empty_haystack() {
            // we have to help with type inference here
            let empty_haystack: &[char; 0] = &[];
            assert!(kmp_match(&['a', 'b', 'c'], empty_haystack).is_empty());
        }

        #[test]
        fn empty_both() {
            // we have to help with type inference here
            let empty_needle: &[char; 0] = &[];
            let empty_haystack: &[char; 0] = &[];
            assert_eq!(vec![0], kmp_match(empty_needle, empty_haystack));
        }

        #[test]
        fn needle_longer_haystack() {
            assert!(kmp_match(&['a', 'b', 'c'], &['a', 'b']).is_empty());
        }
    }

    mod find {
        use crate::{KmpMatchable, KmpPattern, KmpSearchable};

        fn kmp_find<N, H>(needle: &[N], haystack: &[H]) -> Option<usize>
        where
            N: KmpSearchable + KmpMatchable<H>,
        {
            let pattern = KmpPattern::new(needle);
            pattern.find(&haystack).next()
        }

        #[test]
        fn basic() {
            assert_eq!(
                Some(6),
                kmp_find(
                    &['a', 'a', 'a', 'b'],
                    &['a', 'a', 'a', 'a', 'a', 'a', 'a', 'a', 'a', 'b']
                )
            )
        }

        #[test]
        fn empty_needle() {
            assert_eq!(
                Some(0),
                kmp_find::<char, _>(&[], &['a', 'b', 'c', 'd', 'e'])
            );
        }

        #[test]
        fn empty_haystack() {
            assert_eq!(None, kmp_find(&['a', 'b', 'c'], &[]));
        }

        #[test]
        fn empty_both() {
            assert_eq!(Some(0), kmp_find::<char, char>(&[], &[]));
        }

        #[test]
        fn needle_longer_haystack() {
            assert_eq!(None, kmp_find(&['a', 'b', 'c'], &['a', 'b']));
        }
    }

    mod table {
        use crate::{kmp_table as kd, KmpSearchable};

        fn kmp_table<T: KmpSearchable>(needle: &[T]) -> Vec<usize> {
            kd(needle).iter().map(|x| x.needle).collect()
        }

        #[test]
        fn basic() {
            assert_eq!(kmp_table(&['a', 'a', 'a', 'b']), vec![0, 1, 2, 0]);
        }

        #[test]
        fn generation() {
            let empty_needle: &[char; 0] = &[];
            assert!(kmp_table(empty_needle).is_empty());
        }

        #[test]
        fn repeating() {
            assert_eq!(vec![0, 1, 2, 3, 4], kmp_table(&['a', 'a', 'a', 'a', 'a']));
        }

        #[test]
        fn boolean() {
            assert_eq!(
                vec![0, 0, 1, 1, 2, 0, 1],
                kmp_table(&[true, false, true, true, false, false, true])
            );
        }

        #[test]
        fn multiple_chars() {
            assert_eq!(
                vec![0, 0, 1, 0, 1, 2, 3, 2],
                kmp_table(&['a', 'b', 'a', 'c', 'a', 'b', 'a', 'b'])
            );
        }

        #[test]
        fn two_chars_with_repetitions() {
            assert_eq!(
                vec![0, 1, 2, 0, 1, 2, 3, 3, 3, 4],
                kmp_table(&['a', 'a', 'a', 'b', 'a', 'a', 'a', 'a', 'a', 'b'])
            );
        }
    }
}
