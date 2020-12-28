use crate::util::slice::Slice;
use std::sync::atomic::{AtomicUsize, Ordering, AtomicPtr};
use core::mem;

use super::skiplist::{Node, MAX_HEIGHT, MAX_NODE_SIZE};
use std::ptr::slice_from_raw_parts_mut;
use std::slice;

pub trait Arena {
    /// Allocate memory for a node by given height.
    /// This method allocates a Node size + height * ptr ( u64 ) memory area.
    // TODO: define the potential errors and return Result<Error, *mut Node> instead of raw pointer
    fn alloc_node(&self, height: usize) -> *mut Node;

    fn alloc_bytes(&self, data: &Slice) -> u32;

    fn get(&self, offset: usize, count: usize) -> Slice;

    fn has_room_for(&self, size: usize) -> bool;

    fn memory_used(&self)->usize;

    fn size(&self)->usize;
}

/// AggressiveArena is a memory pool for allocating and handling Node memory dynamically.
/// Unlike CommonArena, this simplify the memory handling by aggressively pre-allocating the total fixed memory
/// so it's caller's responsibility to ensure the room before allocating.
pub struct AggressiveArena {
    // indicates that how many memories has been allocated actually
    pub offset: AtomicUsize,
    pub mem: Vec<u8>,
}

impl AggressiveArena {
    /// Create an AggressiveArena with given cap.
    /// This function will allocate a cap size memory block directly for further usage
    pub fn new(cap: usize) -> AggressiveArena {
        AggressiveArena {
            offset: AtomicUsize::new(0),
            mem: Vec::<u8>::with_capacity(cap),
        }
    }

    pub(super) fn display_all(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(self.mem.capacity());
        unsafe {
            let ptr = self.mem.as_ptr();
            for i in 0..self.offset.load(Ordering::Acquire) {
                let p = ptr.add(i) as *mut u8;
                result.push(*p);
            }
        }
        result
    }
}

impl Arena for AggressiveArena {
    fn alloc_node(&self, height: usize) -> *mut Node {
        let ptr_size = mem::size_of::<*mut u8>();
        // truncate node size to reduce waste
        let used_node_size = MAX_NODE_SIZE - (MAX_HEIGHT - height) * ptr_size;
        let n = self.offset.fetch_add(used_node_size, Ordering::SeqCst);
        unsafe {
            let node_ptr = self.mem.as_ptr().add(n) as *mut u8;
            // get the actually to-be-used memory of node and spilt it into 2 parts:
            // node part: the Node struct
            // next parts: the pre allocated memory used by elements of next_nodes
            let (node_part, next_parts) = slice::from_raw_parts_mut(node_ptr, used_node_size)
                .split_at_mut(used_node_size - height * ptr_size);
            let node = node_part.as_mut_ptr() as *mut Node;
            // FIXME: Box::from_raw can be unsafe when releasing memory
            let next_nodes = Box::from_raw(slice::from_raw_parts_mut(
                next_parts.as_mut_ptr() as *mut AtomicPtr<Node>,
                height,
            ));

            (*node).height = height;
            (*node).next_nodes = next_nodes;
            node
        }
    }

    fn alloc_bytes(&self, data: &Slice) -> u32 {
        let start = self.offset.fetch_add(data.size(), Ordering::SeqCst);
        unsafe {
            let ptr = self.mem.as_ptr().add(start) as *mut u8;
            for (i, b) in data.to_slice().iter().enumerate() {
                let p = ptr.add(i) as *mut u8;
                p.replace(*b);
            }
        }
        start as u32
    }

    fn get(&self, start: usize, count: usize) -> Slice {
        let o = self.offset.load(Ordering::Acquire);
        if start + count > o {
            panic!(
                "[arena] try to get data from [{}] to [{}] but max offset is [{}]",
                start,
                start + count,
                o
            );
        }
        let mut result = Vec::with_capacity(count);
        unsafe {
            let ptr = self.mem.as_ptr().add(start) as *mut u8;
            for i in 0..count {
                let p = ptr.add(i) as *mut u8;
                result.push(*p);
            }
        }
        Slice::from(result)
    }

    #[inline]
    fn has_room_for(&self, size: usize) -> bool {
        self.size() - self.memory_used() >= size
    }

    #[inline]
    fn memory_used(&self) -> usize {
        self.offset.load(Ordering::Acquire)
    }

    #[inline]
    fn size(&self) -> usize {
        self.mem.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn new_default_arena() -> AggressiveArena {
        AggressiveArena::new(64 << 20)
    }

    #[test]
    fn test_new_arena() {
        let cap = 200;
        let arena = AggressiveArena::new(cap);
        assert_eq!(arena.memory_used(), 0);
        assert_eq!(arena.size(), cap);
    }

    #[test]
    fn test_alloc_single_node() {
        let arena = new_default_arena();
        let node = arena.alloc_node(MAX_HEIGHT);
        unsafe {
            assert_eq!((*node).height, MAX_HEIGHT);
            assert_eq!((*node).next_nodes.len(), MAX_HEIGHT);
            assert_eq!((*node).key_size, 0);
            assert_eq!((*node).key_offset, 0);
            assert_eq!((*node).value_size, 0);
            assert_eq!((*node).value_offset, 0);

            // dereference and assigning should work
            let u8_ptr = node as *mut u8;
            (*node).key_offset = 1;
            let key_offset_ptr = u8_ptr.add(0);
            assert_eq!(*key_offset_ptr, 1);
            (*node).key_size = 2;
            let key_size_ptr = u8_ptr.add(8);
            assert_eq!(*key_size_ptr, 2);
            (*node).value_offset = 3;
            let value_offset_ptr = u8_ptr.add(16);
            assert_eq!(*value_offset_ptr, 3);
            (*node).value_size = 4;
            let value_size_ptr = u8_ptr.add(24);
            assert_eq!(*value_size_ptr, 4);

            // the value of data ptr in 'next_nodes' slice must be the beginning pointer of first element
            let next_nodes_ptr = u8_ptr
                .add(mem::size_of::<Node>() - mem::size_of::<Box<[AtomicPtr<Node>]>>())
                as *mut u64;
            let first_element_ptr = u8_ptr.add(mem::size_of::<Node>());
            assert_eq!(
                "0x".to_owned() + &format!("{:x}", *next_nodes_ptr),
                format!("{:?}", first_element_ptr)
            );
        }
    }

    #[test]
    fn test_alloc_nodes() {
        let arena = new_default_arena();
        let node1 = arena.alloc_node(4);
        let node2 = arena.alloc_node(MAX_HEIGHT);
        unsafe {
            // node1 and node2 should be neighbor in memory
            let struct_tail = node1.add(1) as *mut *mut Node;
            let next_tails = struct_tail.add(4);
            assert_eq!(next_tails as *mut Node, node2);
        };
    }

    #[test]
    fn test_alloc_bytes_concurrency() {
        let arena = Arc::new(AggressiveArena::new(500));
        let node = arena.alloc_node(1);
        let results = Arc::new(Mutex::new(vec![]));
        let mut tests = vec![vec![1u8, 2, 3, 4, 5], vec![6u8, 7, 8, 9], vec![10u8, 11]];
        for t in tests
            .drain(..)
            .enumerate()
            .map(|(i, test)| {
                let cloned_arena = arena.clone();
                let cloned_results = results.clone();
                thread::spawn(move || {
                    let offset = cloned_arena.alloc_bytes(&Slice::from(test.clone())) as usize;
                    cloned_results.lock().unwrap().push((i, offset, test.clone()));
                })
            })
            .collect::<Vec<_>>()
        {
            t.join().unwrap();
        }
        let mem_ptr = arena.mem.as_ptr();
        for (index, offset, expect) in results.lock().unwrap().drain(..) {
            unsafe {
                let ptr = mem_ptr.add(offset) as *mut u8;
                for (i, b) in expect.iter().enumerate() {
                    let inmem_b = ptr.add(i);
                    assert_eq!(*inmem_b, *b);
                }
            }
        }
    }
}