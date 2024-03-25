use miniscript::iter::{Tree, TreeLike};

/// View of a slice as a balanced binary tree of its elements.
/// The slice must be nonempty.
#[derive(Debug, Copy, Clone)]
pub struct BTreeSlice<'a, A>(&'a [A]);

impl<'a, A> BTreeSlice<'a, A> {
    /// View the slice as a balanced binary tree of its elements.
    ///
    /// ## Panics
    ///
    /// The slice is empty.
    pub fn from_slice(slice: &'a [A]) -> Self {
        assert!(!slice.is_empty(), "Slice must be nonempty");
        Self(slice)
    }
}

impl<'a, A: Clone> BTreeSlice<'a, A> {
    /// Fold the tree in post order, using the binary function `f`.
    pub fn fold<F>(self, f: F) -> A
    where
        F: Fn(A, A) -> A,
    {
        debug_assert!(!self.0.is_empty());

        let mut output = vec![];
        for item in self.post_order_iter() {
            match item.child_indices.len() {
                2 => {
                    let r = output.pop().unwrap();
                    let l = output.pop().unwrap();
                    output.push(f(l, r));
                }
                n => {
                    debug_assert!(n == 0);
                    debug_assert!(item.node.0.len() == 1);
                    output.push(item.node.0[0].clone());
                }
            }
        }

        debug_assert!(output.len() == 1);
        output.pop().unwrap()
    }
}

impl<'a, A: Clone> TreeLike for BTreeSlice<'a, A> {
    fn as_node(&self) -> Tree<Self> {
        match self.0.len() {
            0 => unreachable!("Empty slice"),
            1 => Tree::Nullary,
            n => {
                let next_pow2 = n.next_power_of_two();
                debug_assert!(0 < next_pow2 / 2);
                debug_assert!(0 < n - next_pow2 / 2);
                let half = next_pow2 / 2;
                let left = BTreeSlice::from_slice(&self.0[..half]);
                let right = BTreeSlice::from_slice(&self.0[half..]);
                Tree::Binary(left, right)
            }
        }
    }
}

/// Partition of a slice into blocks of (lengths of) powers of two.
///
/// The blocks start at (length) `N` and decrease to one in order.
/// Depending on the (length of the) slice, some blocks might be empty.
///
/// A partition forms a binary tree:
///
/// 1. A slice of length `l = 1` is a leaf
/// 2. A slice of length `l ≥ N` is a parent:
///     1. Left child: The block of the first `N` elements
///     2. Right child: The partition of the remaining `l - N` elements
/// 3. A slice of length `1 < l < N` is a parent:
///     1. Left child: The empty block
///     2. Right child: The partition of the remaining `l` elements
#[derive(Debug, Copy, Clone)]
pub enum Partition<'a, A> {
    Leaf(&'a [A]),
    Parent { slice: &'a [A], block_len: usize },
}

impl<'a, A> Partition<'a, A> {
    /// Partition the `slice` into blocks starting at the given `block_len`.
    ///
    /// ## Panics
    ///
    /// The `block_len` is not a power of two.
    ///
    /// The `block_len` is not large enough to partition the slice (2 * `block_len` ≤ slice length).
    pub fn from_slice(slice: &'a [A], block_len: usize) -> Self {
        assert!(
            block_len.is_power_of_two(),
            "The block length must be a power of two"
        );
        assert!(
            slice.len() < block_len * 2,
            "The block length must be large enough to partition the slice"
        );
        match block_len {
            1 => Self::Leaf(slice),
            _ => Self::Parent { slice, block_len },
        }
    }
}

impl<'a, A: Clone> Partition<'a, A> {
    /// Check if the partition is complete.
    ///
    /// A complete partition contains no empty blocks.
    pub fn is_complete(&self) -> bool {
        match self {
            Partition::Leaf(slice) => {
                debug_assert!(slice.len().is_power_of_two());
                slice.len() == 1
            }
            Partition::Parent { slice, block_len } => {
                debug_assert!(slice.len() < block_len * 2);
                slice.len() + 1 == block_len * 2
            }
        }
    }

    /// Fold the tree of blocks in post-order.
    ///
    /// There are two steps:
    /// 1. Function `f` converts each block (leaf node) into an output value.
    /// 2. Function `g` joins the outputs of each leaf in post-order.
    ///
    /// Function `f` must handle empty blocks if the partition is not complete.
    pub fn fold<B, F, G>(self, f: F, g: G) -> B
    where
        F: Fn(&[A]) -> B,
        G: Fn(B, B) -> B,
    {
        let mut output = vec![];
        for item in self.post_order_iter() {
            match item.node {
                Partition::Leaf(slice) => {
                    output.push(f(slice));
                }
                Partition::Parent { .. } => {
                    let r = output.pop().unwrap();
                    let l = output.pop().unwrap();
                    output.push(g(l, r));
                }
            }
        }

        debug_assert!(output.len() == 1);
        output.pop().unwrap()
    }
}

#[rustfmt::skip]
impl<'a, A: Clone> TreeLike for Partition<'a, A> {
    fn as_node(&self) -> Tree<Self> {
        match self {
            Self::Leaf(..) => Tree::Nullary,
            Self::Parent { slice, block_len } => {
                debug_assert!(2 <= *block_len);
                let (l, r) = if slice.len() < *block_len {
                    (
                        Self::Leaf(&[]),
                        Self::from_slice(slice, block_len / 2),
                    )
                } else {
                    (
                        Self::Leaf(&slice[..*block_len]),
                        Self::from_slice(&slice[*block_len..], block_len / 2),
                    )
                };
                Tree::Binary(l, r)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn fold_btree_slice() {
        let slice_output: [(&[&str], &str); 8] = [
            (&["a"], "a"),
            (&["a", "b"], "(ab)"),
            (&["a", "b", "c"], "((ab)c)"),
            (&["a", "b", "c", "d"], "((ab)(cd))"),
            (&["a", "b", "c", "d", "e"], "(((ab)(cd))e)"),
            (&["a", "b", "c", "d", "e", "f"], "(((ab)(cd))(ef))"),
            (&["a", "b", "c", "d", "e", "f", "g"], "(((ab)(cd))((ef)g))"),
            (&["a", "b", "c", "d", "e", "f", "g", "h"], "(((ab)(cd))((ef)(gh)))"),
        ];
        let concat = |a: String, b: String| format!("({a}{b})");

        for (slice, expected_output) in slice_output {
            let vector: Vec<_> = slice.iter().map(|s| s.to_string()).collect();
            let tree = BTreeSlice::from_slice(&vector);
            let output = tree.fold(concat);
            assert_eq!(&output, expected_output);
        }
    }
}
