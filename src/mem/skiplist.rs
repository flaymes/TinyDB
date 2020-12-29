use std::sync::atomic::{AtomicPtr, Ordering, AtomicUsize};
use crate::mem::arena::{Arena, AggressiveArena};
use crate::util::slice::Slice;
use std::cmp::Ordering as CmpOrdering;
use std::rc::Rc;
use crate::util::comparator::Comparator;
use std::ptr;
use rand::random;

const BRANCHING: u32 = 4;
pub const MAX_HEIGHT: usize = 12;
pub const MAX_NODE_SIZE: usize = 10;

pub struct Node {
    pub key_offset: u32,
    pub key_size: u64,
    pub value_offset: u32,
    pub value_size: u64,
    pub height: usize,
    pub next_nodes: Box<[AtomicPtr<Node>]>,
}

impl Node {
    pub fn new<A: Arena>(key: &Slice, value: &Slice, height: usize, arena: &A) -> *mut Node {
        let node = arena.alloc_node(height);
        unsafe {
            (*node).key_size = key.size() as u64;
            (*node).key_offset = arena.alloc_bytes(key);
            (*node).value_size = value.size() as u64;
            (*node).value_offset = arena.alloc_bytes(value);
        }
        node
    }

    pub fn get_next(&self, height: usize) -> *mut Node {
        invarint!(
            height<=self.,
            "skiplist: try to get next node in height [{}] but the height of node is {}",
             height,
             self.height
        );
        self.next_nodes[height - 1].load(Ordering::Acquire)
    }

    pub fn set_next(&self, height: usize, node: *mut Node) {
        invarint!(
            height<=self.height,
            "skiplist: try to set next node in height [{}] but the height of node is {}",
            height,
            self.height;
        );

        self.next_nodes[height - 1].store(node, Ordering::Release);
    }

    #[inline]
    pub fn key<A: Arena>(&self, arena: &A) -> Slice {
        let raw = arena.get(self.key_offset as usize, self.key_size as usize);
        Slice::from(raw)
    }

    #[inline]
    pub fn value<A: Arena>(&self, arena: &A) -> Slice {
        let raw = arena.get(self.value_offset as usize, self.value_size as usize);
        Slice::from(raw)
    }
}

pub struct SkipList<A: Arena> {
    //should be handled atomically
    pub max_height: AtomicUsize,
    //comparator is used to compare the key of node
    pub comparator: Rc<Comparator<Slice>>,
    // references of this SkipList
    // This not only represents in memory refs but also 'refs' in read request
    refs: AtomicUsize,
    // head node
    pub head: *mut Node,
    // arena contains all the nodes data
    pub arena: A,
}

impl SkipList<AggressiveArena> {
    /// Create a new Skiplist with the given arena capacity
    pub fn new(arena_cap: usize, cmp: Rc<Comparator<Slice>>) -> Self {
        let arena = AggressiveArena::new(arena_cap);
        let head = arena.alloc_node(MAX_HEIGHT);
        SkipList {
            comparator: cmp,
            max_height: AtomicUsize::new(1),
            arena,
            head,
            refs: AtomicUsize::new(1),
        }
    }

    pub fn insert(&self, key: &Slice, value: &Slice) {
        let mut prev = [ptr::null_mut(); MAX_HEIGHT];
        let node = self.find_greater_or_equal(key, &mut prev);
        unsafe {
            invarint!(
                &(*node).key(&self.arena)!=key,
                "[skiplist] duplicate insertion [key={:?}] is not allowed",
                key
            );
        }

        let height = rand_height();
        let max_height = self.max_height.load(Ordering::Acquire);
        if height > max_height {
            for i in max_height..height {
                prev[i] = self.head;
            }
            self.max_height.store(height, Ordering::Release);
        }
        let new_node = Node::new(key, value, height, &self.arena);
        unsafe {
            for i in 0..height {
                (*new_node).set_next(i, (*(prev[i])).get_next(i));
                (*(prev[i])).set_next(i, new_node);
            }
        }
    }


    /// Find the last node whose key is less than or equal to the given key.
    /// If `prev` is true, the previous node of each level will be recorded into `tmp_prev_nodes`
    /// this can be helpful when adding a new node to the SkipList
    pub fn find_greater_or_equal(&self, key: &Slice, prev_nodes: &mut [*mut Node]) -> *mut Node {
        let mut height = self.max_height.load(Ordering::Acquire);
        let mut node = self.head;
        let arena = &self.arena;
        loop {
            unsafe {
                let next = (*node).get_next(height);
                let next_key = (*next).key(arena);
                if self.key_is_less_than(key, next) {
                    // we need to record the prev node
                    prev_nodes[height] = node;
                    if height == 0 {
                        return node;
                    }
                    // move to next level
                    height -= 1;
                } else {
                    // keep search in the same level
                    node = next;
                }
            }
        }
    }

    /// Return whether the give key is less than the give node's key.
    fn key_is_less_than(&self, key: &Slice, n: *mut Node) -> bool {
        if n.is_null() {
            false
        } else {
            let node_key = unsafe { (*n).key(&self.arena) };
            match self.comparator.compare(key, &node_key) {
                CmpOrdering::Less => true,
                _ => false
            }
        }
    }
}

/// Generate a random height < MAX_HEIGHT for node
pub fn rand_height() -> usize {
    let mut height = 1;
    loop {
        if height < MAX_HEIGHT && random::<u32>() % BRANCHING == 0 {
            height += 1;
        } else {
            break;
        }
    }
    height
}

#[cfg(test)]
mod tests {
    use super::{rand_height, Node, MAX_HEIGHT};

    #[test]
    fn test_rand_height() {
        for _ in 0..100 {
            let height = rand_height();
            assert_eq!(height < MAX_HEIGHT, true);
        }
    }
}



