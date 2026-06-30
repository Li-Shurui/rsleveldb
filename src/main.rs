use rsleveldb::Arena;

fn main() {
    let mut arena = Arena::new();

    // Basic allocation
    let data = arena.allocate(256);
    println!("allocated {} bytes", data.len());
    for (i, byte) in data.iter_mut().enumerate() {
        *byte = i as u8;
    }

    // Aligned allocation
    let aligned = arena.allocate_aligned(128);
    let addr = aligned.as_ptr() as usize;
    println!(
        "aligned allocation: {} bytes at 0x{:x} ({} aligned)",
        aligned.len(),
        addr,
        addr % 8
    );

    // Multiple allocations across blocks
    {
        let chunk = arena.allocate(5000);
        println!("large allocation: {} bytes", chunk.len());
        chunk[0] = 0xAB;
    }

    {
        let another = arena.allocate(64);
        println!("another allocation: {} bytes", another.len());
    }

    println!("total memory usage: {} bytes", arena.memory_usage());
}
