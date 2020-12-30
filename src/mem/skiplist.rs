use super::arena::*;
use crate::util::slice::Slice;
use crate::util::comparator::Comparator;

use std::sync::atomic::{AtomicPtr, Ordering, AtomicUsize};
use std::cmp::Ordering as CmpOrdering;
use std::rc::Rc;
use std::ptr;
use rand::random;

const BRANCHING: u32 = 4;
pub const MAX_HEIGHT: usize = 12;
pub const MAX_NODE_SIZE: usize = 10;

#[derive(Debug)]
#[repr(C)]
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
            height<=self.height,
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
            self.height
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
        let mut level = self.max_height.load(Ordering::Acquire);
        let mut node = self.head;
        let arena = &self.arena;
        loop {
            unsafe {
                let next = (*node).get_next(level);
                if self.key_is_less_than(key, next) {
                    // we need to record the prev node
                    prev_nodes[level - 1] = node;
                    if level == 1 {
                        return next;
                    }
                    // move to next level
                    level -= 1;
                } else {
                    // keep search in the same level
                    node = next;
                }
            }
        }
    }

    pub fn find_less_than(&self, key: &Slice) -> *mut Node {
        let mut level = self.max_height.load(Ordering::Acquire);
        let mut node = self.head;
        let arena = &self.arena;
        loop {
            unsafe {
                let next = (*node).get_next(level);
                if next.is_null()
                    || self.comparator.compare(&((*next)).key(arena), key) != CmpOrdering::Less {
                    if level == 1 {
                        return node;
                    } else {
                        level -= 1;
                    }
                } else {
                    node = next;
                }
            }
        }
    }

    pub fn find_last(&self) -> *mut Node {
        let mut level = self.max_height.load(Ordering::Acquire);
        let mut node = self.head;
        let arena = &self.arena;
        loop {
            unsafe {
                let next = (*node).get_next(level);
                if next.is_null() {
                    if level == 1 {
                        return node;
                    }
                    level -= 1;
                } else {
                    node = next;
                }
            }
        }
    }

    /// Return whether the give key is less than the give node's key.
    fn key_is_less_than(&self, key: &Slice, n: *mut Node) -> bool {
        if n.is_null() {
            true
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
    use super::{rand_height, MAX_HEIGHT};
    use super::*;
    use crate::util::comparator::BytewiseComparator;
    use std::ptr;
    use std::rc::Rc;

    fn new_test_skl() -> Skiplist<AggressiveArena> {
        Skiplist::new(64 << 20, Rc::new(BytewiseComparator::new()))
    }
    #[test]
    fn test_rand_height() {
        for _ in 0..100 {
            let height = rand_height();
            assert_eq!(height < MAX_HEIGHT, true);
        }
    }

    #[test]
    fn test_key_is_less_than() {
        let skl = new_test_skl();
        let vec = vec![1u8, 2u8, 3u8];
        let key = Slice::from(vec.as_slice());
        // return false if node is nullptr
        assert_eq!(false, skl.key_is_less_than(&key, ptr::null_mut()));

        let n = Node::new(
            &Slice::from(vec![1u8, 2u8].as_slice()),
            &Slice::from(""),
            1,
            &skl.arena,
        );
        assert_eq!(false, skl.key_is_less_than(&key, n));

        let n2 = Node::new(
            &Slice::from(vec![1u8, 2u8, 4u8].as_slice()),
            &Slice::from(""),
            1,
            &skl.arena,
        );
        assert_eq!(true, skl.key_is_less_than(&key, n2));
    }

    #[test]
    fn test_find_greater_or_equal() {
        let skl = new_test_skl();
        skl.max_height.store(5, Ordering::Release);
        let value = Slice::from("");
        let n1 = Node::new(&Slice::from("key1"), &value, 5, &skl.arena);
        let n2 = Node::new(&Slice::from("key3"), &value, 1, &skl.arena);
        let n3 = Node::new(&Slice::from("key5"), &value, 2, &skl.arena);
        let n4 = Node::new(&Slice::from("key7"), &value, 4, &skl.arena);
        let n5 = Node::new(&Slice::from("key9"), &value, 3, &skl.arena);

        // Manually construct a skiplist
        // TODO: use a easier way to construct the skiplist
        unsafe {
            for i in 0..5 {
                (*skl.head).next_nodes[i].store(n1, Ordering::Release);
            }
            (*n1).set_next(1, n2);
            (*n1).set_next(2, n3);
            (*n1).set_next(4, n4);
            (*n1).set_next(3, n5);
            (*n2).set_next(1, n3);
            (*n3).set_next(1, n4);
            (*n3).set_next(2, n4);
            (*n4).set_next(1, n5);
            (*n4).set_next(2, n5);
            (*n4).set_next(3, n5);
        }

        let mut prev_nodes = vec![ptr::null_mut(); 5];
        let target_key = Slice::from("key4");
        let res = skl.find_greater_or_equal(&target_key, &mut prev_nodes);
        assert_eq!(res, n3);
        // prev_nodes should be correct
        assert_eq!(prev_nodes[0], n2);
        for node in prev_nodes[1..5].iter() {
            assert_eq!(*node, n1);
        }
    }

    #[test]
    fn test_find_less_than() {}

    #[test]
    fn test_basic() {}
}



