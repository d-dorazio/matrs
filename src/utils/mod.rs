//! some handy utils.

use std::collections::HashMap;

use std::hash::Hash;

/// Build a `HashMap` from the keys in the iterator to the number of its
/// occurences.
pub fn build_hashmap_counter<K, I>(it: I) -> HashMap<K, u64>
where
    K: Eq + Hash,
    I: Iterator<Item = K>,
{
    let mut map = HashMap::new();

    for k in it {
        *map.entry(k).or_insert(0) += 1;
    }

    map
}

/// Split the given vector at the given index and return a vector of all
/// elements before at, the element at the given index and all the elements
/// after.
pub fn split_element_at<T>(mut v: Vec<T>, at: usize) -> (Vec<T>, Option<T>, Vec<T>) {
    if v.is_empty() {
        return (vec![], None, vec![]);
    }

    let right = v.split_off(at + 1);
    let elem = v.pop();

    (v, elem, right)
}

pub mod ksmallest;
pub mod ordwrapper;

pub use self::ksmallest::{ksmallest, ksmallest_by, ksmallest_by_key};
pub use self::ordwrapper::OrdWrapper;

#[cfg(test)]
mod test {
    use super::split_element_at;

    extern crate proptest;

    use std::iter;

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(50))]
        #[test]
        fn prop_split_element_at_built_from_parts(
            left in proptest::collection::vec(0_u8..255, 0..100),
            elem in (0_u8..),
            right in proptest::collection::vec(0_u8..255, 0..100)
        ) {
            let composed = left
                .iter()
                .chain(iter::once(&elem))
                .chain(right.iter())
                .cloned()
                .collect();

            let (l, e, r) = split_element_at(composed, left.len());

            assert_eq!(e, Some(elem));
            assert_eq!(l, left);
            assert_eq!(r, right);
        }
    }
}
