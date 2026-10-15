use std::sync::Barrier;

use codex_code_mode_runtime::V8JitMode;
use codex_code_mode_runtime::initialize_v8;

#[test]
fn concurrent_first_default_allocators_do_not_race_process_initialization() {
    // This integration binary keeps the process-wide allocator cold until all
    // workers have completed V8 initialization and are ready to create it.
    let barrier = Barrier::new(32);
    std::thread::scope(|scope| {
        for _ in 0..32 {
            scope.spawn(|| {
                initialize_v8(V8JitMode::Enabled).expect("initialize V8");
                barrier.wait();
                let _allocator = v8::new_default_allocator();
                barrier.wait();
            });
        }
    });
}
