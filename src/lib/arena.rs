use std::sync::atomic::{AtomicUsize, Ordering};

const BLOCK_SIZE: usize = 4096;

/// A bump-style arena allocator.
///
/// Memory is allocated in blocks and freed all at once when the arena
/// is dropped.  This mirrors the semantics of LevelDB's `leveldb::Arena`.
pub struct Arena {
    blocks: Vec<Vec<u8>>,
    current_block_index: usize,
    alloc_ptr: usize,
    memory_usage: AtomicUsize,
}

impl Arena {
    /// Create a new, empty arena.
    pub fn new() -> Self {
        Arena {
            blocks: Vec::new(),
            current_block_index: 0,
            alloc_ptr: 0,
            memory_usage: AtomicUsize::new(0),
        }
    }

    /// Allocate a block of `bytes` bytes with default alignment.
    ///
    /// The returned slice is valid until the arena is dropped.  While
    /// the slice is alive you may not call `allocate` or
    /// `allocate_aligned` again (this mirrors Rust's normal borrowing
    /// rules).
    pub fn allocate(&mut self, bytes: usize) -> &mut [u8] {
        assert!(bytes > 0, "Arena does not support zero-byte allocations");

        if let Some(block) = self.blocks.last_mut() {
            let remaining = block.len() - self.alloc_ptr;
            if bytes <= remaining {
                let start = self.alloc_ptr;
                self.alloc_ptr += bytes;
                // SAFETY: `start..start+bytes` is within the bounds of `block`
                // and `block` is stored in `self.blocks`, so the slice outlives
                // this function.
                return unsafe {
                    std::slice::from_raw_parts_mut(block.as_mut_ptr().add(start), bytes)
                };
            }
        }

        self.allocate_fallback(bytes)
    }

    /// Allocate a block of `bytes` bytes aligned to at least
    /// `align_of::<*const u8>()` (typically 8 bytes on 64-bit).
    ///
    /// If the remaining space in the current block is not large enough
    /// to satisfy the aligned request, a fresh block is allocated
    /// instead (matching LevelDB's behaviour).
    pub fn allocate_aligned(&mut self, bytes: usize) -> &mut [u8] {
        assert!(bytes > 0, "Arena does not support zero-byte allocations");
        let align = std::mem::align_of::<*const u8>().max(8);

        if let Some(block) = self.blocks.last_mut() {
            let base_ptr = block.as_mut_ptr() as usize;
            let current_ptr = base_ptr + self.alloc_ptr;
            let current_mod = current_ptr & (align - 1);
            let slop = if current_mod == 0 { 0 } else { align - current_mod };
            let needed = bytes + slop;
            let remaining = block.len() - self.alloc_ptr;

            if needed <= remaining {
                let start = self.alloc_ptr + slop;
                self.alloc_ptr += needed;
                // SAFETY: `start..start+bytes` is within the bounds of `block`.
                return unsafe {
                    std::slice::from_raw_parts_mut(block.as_mut_ptr().add(start), bytes)
                };
            }
        }

        self.allocate_fallback(bytes)
    }

    /// Return an estimate of the total memory usage of the arena.
    pub fn memory_usage(&self) -> usize {
        self.memory_usage.load(Ordering::Relaxed)
    }
}

impl Arena {
    fn allocate_fallback(&mut self, bytes: usize) -> &mut [u8] {
        if bytes > BLOCK_SIZE / 4 {
            let block = vec![0u8; bytes];
            self.memory_usage.fetch_add(
                bytes + std::mem::size_of::<usize>(),
                Ordering::Relaxed,
            );
            self.blocks.push(block);
            self.current_block_index = self.blocks.len() - 1;
            self.alloc_ptr = bytes;
            let block = self.blocks.last_mut().unwrap();
            // SAFETY: `block` is valid and `bytes <= block.len()`
            unsafe { std::slice::from_raw_parts_mut(block.as_mut_ptr(), bytes) }
        } else {
            let block = vec![0u8; BLOCK_SIZE];
            self.memory_usage.fetch_add(
                BLOCK_SIZE + std::mem::size_of::<usize>(),
                Ordering::Relaxed,
            );
            self.blocks.push(block);
            self.current_block_index = self.blocks.len() - 1;
            self.alloc_ptr = bytes;
            let block = self.blocks.last_mut().unwrap();
            // SAFETY: `block` is valid and `bytes <= block.len()`
            unsafe { std::slice::from_raw_parts_mut(block.as_mut_ptr(), bytes) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_arena_has_no_blocks() {
        let arena = Arena::new();
        assert_eq!(arena.memory_usage(), 0);
    }

    #[test]
    fn test_single_allocation() {
        let mut arena = Arena::new();
        let data = arena.allocate(64);
        assert_eq!(data.len(), 64);
        data[0] = 1;
        data[63] = 255;
        assert_eq!(data[0], 1);
        assert_eq!(data[63], 255);
    }

    #[test]
    fn test_multiple_allocations_same_block() {
        let mut arena = Arena::new();
        {
            let a = arena.allocate(100);
            a.fill(1);
        }
        {
            let b = arena.allocate(100);
            b.fill(2);
        }
        {
            let c = arena.allocate(100);
            c.fill(3);
        }
    }

    #[test]
    fn test_aligned_allocation() {
        let mut arena = Arena::new();
        let data = arena.allocate_aligned(64);
        let addr = data.as_ptr() as usize;
        assert_eq!(addr % 8, 0, "allocation should be 8-byte aligned");
        assert_eq!(data.len(), 64);
    }

    #[test]
    fn test_large_allocation_gets_own_block() {
        let mut arena = Arena::new();
        let large = arena.allocate(5000);
        assert_eq!(large.len(), 5000);
        large[0] = 42;
        assert_eq!(large[0], 42);
    }

    #[test]
    fn test_memory_usage_tracks_allocations() {
        let mut arena = Arena::new();
        arena.allocate(128);
        let usage = arena.memory_usage();
        assert!(usage >= 128, "memory usage {} should be >= 128", usage);
    }

    #[test]
    fn test_zero_byte_panics() {
        let mut arena = Arena::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            arena.allocate(0);
        }));
        assert!(result.is_err(), "zero-byte allocation should panic");
    }
}
