//! Writing a profile needs little memory beside the profile itself (al-runner issue 6924: the merged profile of a long
//! run was built as one JSON value and one string, several times its own size, after the result lines and past
//! `--max-memory`). A binary of its own: it counts every allocation of the process with its own global allocator.

use al_testrun::profiling::{AppRef, CodeunitProfile, Collected, Mode, Node, Options, Thread, write};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Duration;

struct Counting;
static NOW: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let n = NOW.fetch_add(l.size(), Relaxed) + l.size();
        PEAK.fetch_max(n, Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        NOW.fetch_sub(l.size(), Relaxed);
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static A: Counting = Counting;

/// A codeunit profile of `n` nodes: a root, then a chain of routines, each with statements.
fn profile(codeunit: u32, n: usize, mode: Mode) -> CodeunitProfile {
    let app = AppRef {
        id: "6b2a1f3e-0000-4000-8000-0000000000aa".into(),
        name: "Calib".into(),
        publisher: "Me".into(),
        version: "1.0.0.0".into(),
    };
    let nodes = (0..n)
        .map(|i| Node {
            parent: i.checked_sub(1),
            children: if i + 1 < n { vec![i + 1] } else { vec![] },
            function: format!("Routine{i}").into(),
            object_type: "CodeUnit",
            keyword: "Codeunit",
            object_name: "Calib".into(),
            object_id: 50100,
            app: app.clone().into(),
            file: Some("src/Calib.al".into()),
            line: 9,
            column: 8,
            calls: 1,
            total_ns: ((n - i) * 1000) as u64,
            child_ns: ((n - i - 1) * 1000) as u64,
            samples: 1,
            sample_ns: 1000,
            frame_name: format!("codeunit Calib.Routine{i}").into(),
            ..Node::default()
        })
        .collect();
    CodeunitProfile {
        codeunit,
        name: format!("Tests {codeunit}"),
        app,
        thread: Thread {
            id: 1,
            label: "Thread 1 (main)".into(),
        },
        sampling: mode == Mode::Sampling,
        start_unix_us: 1_700_000_000_000_000,
        wall_ns: (n * 1000) as u64,
        samples: n as u64,
        nodes,
        tests: vec![("TestA".into(), Duration::from_millis(1))],
    }
}

#[test]
fn a_merged_profile_is_written_without_a_copy_of_it_in_memory() {
    for mode in [Mode::Sampling, Mode::Instrumentation] {
        let c = Collected {
            options: Options {
                mode,
                interval_us: 1000,
                statement_times: true,
            },
            // Chains of 1000 nodes: deep enough to be a real call tree, shallow enough for the writer's recursion.
            codeunits: (0..100).map(|k| profile(50_000 + k, 1000, mode)).collect(),
        };
        let dir = std::env::temp_dir().join(format!("al-profile-mem-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("run.alcpuprofile");
        let held = NOW.load(Relaxed);
        PEAK.store(held, Relaxed);
        write::write(&path, &c, &|_| None, &|| false).unwrap();
        let extra = PEAK.load(Relaxed) - held;
        let size = std::fs::metadata(&path).unwrap().len() as usize;
        let text = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["nodes"].as_array().unwrap().len(), 1 + 1 + 100 * 1000, "{mode:?}");
        drop((text, v));
        std::fs::remove_dir_all(&dir).ok();
        // The file is about 50 MB; writing it may hold a few small buffers and the subtree sizes, not the document.
        assert!(
            extra < 16 << 20,
            "{mode:?}: writing a {} MB profile held {} MB more",
            size >> 20,
            extra >> 20
        );
    }
}
