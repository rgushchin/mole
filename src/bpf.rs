// use anyhow::{bail, Result};
// use libbpf_rs::PerfBufferBuilder;
// use plain::Plain;
// use std::collections::HashMap;
// use std::str;
// use std::time::{Duration, Instant};
// use structopt::StructOpt;

// #[path = "bpf/.output/mole.skel.rs"]
// mod mole;
// use mole::*;

// /// Trace high run queue latency
// #[derive(Debug, StructOpt)]
// struct Command {
//     /// Trace latency higher than this value
//     #[structopt(default_value = "10000")]
//     latency: u64,
//     /// Process PID to trace
//     #[structopt(default_value = "0")]
//     pid: i32,
//     /// Thread TID to trace
//     #[structopt(default_value = "0")]
//     tid: i32,
//     /// Verbose debug output
//     #[structopt(short, long)]
//     verbose: bool,
// }

// unsafe impl Plain for mole_bss_types::event {}

// fn bump_memlock_rlimit() -> Result<()> {
//     let rlimit = libc::rlimit {
//         rlim_cur: 128 << 20,
//         rlim_max: 128 << 20,
//     };

//     if unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlimit) } != 0 {
//         bail!("Failed to increase rlimit");
//     }

//     Ok(())
// }

// struct Task {
//     pid: i32,
//     comm: String,
// }

// #[derive(Default)]
// struct Context {
//     tasks: HashMap<i32, Task>,
//     wakeups: HashMap<(i32, i32), u64>,
//     nr_wakeups: u64,
// }

// fn handle_event(ctx: &mut Context, _cpu: i32, data: &[u8]) {
//     let mut event = mole_bss_types::event::default();

//     plain::copy_from_bytes(&mut event, data).expect("Data buffer was too short");

//     ctx.tasks.entry(event.pid1).or_insert_with(|| Task {
//         pid: event.pid1,
//         comm: String::from(str::from_utf8(&event.comm1).unwrap()),
//     });

//     ctx.tasks.entry(event.pid2).or_insert_with(|| Task {
//         pid: event.pid2,
//         comm: String::from(str::from_utf8(&event.comm2).unwrap()),
//     });

//     let wakeup = ctx.wakeups.entry((event.pid1, event.pid2)).or_insert(0);
//     *wakeup += 1;

//     ctx.nr_wakeups += 1;
// }

// fn handle_lost_events(cpu: i32, count: u64) {
//     eprintln!("Lost {} events on CPU {}", count, cpu);
// }

// fn main2() -> Result<()> {
//     let opts = Command::from_args();

//     let mut skel_builder = MoleSkelBuilder::default();
//     if opts.verbose {
//         skel_builder.obj_builder.debug(true);
//     }

//     bump_memlock_rlimit()?;
//     let mut open_skel = skel_builder.open()?;

//     // Write arguments into prog
//     open_skel.rodata().min_us = opts.latency;
//     open_skel.rodata().targ_pid = opts.pid;
//     open_skel.rodata().targ_tgid = opts.tid;

//     // Begin tracing
//     let mut skel = open_skel.load()?;
//     skel.attach()?;

//     let mut ctx = Context::default();

//     {
//         let perf = PerfBufferBuilder::new(skel.maps_mut().events())
//             .sample_cb(|cpu: i32, data: &[u8]| {
//                 handle_event(&mut ctx, cpu, data);
//             })
//             .lost_cb(handle_lost_events)
//             .build()?;

//         let start = Instant::now();
//         loop {
//             perf.poll(Duration::from_millis(100))?;

//             let now = Instant::now();

//             if now - start > Duration::from_secs(5) {
//                 break;
//             }
//         }
//     }

//     println!("Processing {} events...", ctx.nr_wakeups);

//     let mut sorted: Vec<_> = ctx.wakeups.iter().collect();
//     sorted.sort_unstable_by(|a, b| b.1.cmp(a.1));

//     for ((t1, t2), count) in sorted {
// 	if *count < ctx.nr_wakeups / 100 {
// 	    println!("Skipped...");
// 	    break;
// 	}

// 	let comm1 = &ctx.tasks.get(&t1).unwrap().comm;
// 	let comm2 = &ctx.tasks.get(&t2).unwrap().comm;

// 	println!("{:6} {:16} ({}) -> {:16} ({})", count, comm1, t1, comm2, t2);
//     }

//     Ok(())
// }
