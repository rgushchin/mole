use anyhow::{bail, Result};
use libbpf_rs::PerfBufferBuilder;
use plain::Plain;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[path = "bpf/.output/mole.skel.rs"]
mod mole;
use mole::*;

unsafe impl Plain for mole_bss_types::wake_event {}

fn bump_memlock_rlimit() -> Result<()> {
    let rlimit = libc::rlimit {
        rlim_cur: 128 << 20,
        rlim_max: 128 << 20,
    };

    if unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlimit) } != 0 {
        bail!("Failed to increase rlimit");
    }

    Ok(())
}

pub type Wakeups = HashMap<(u64, u64), u64>;

fn handle_event(ctx: &mut Wakeups, _cpu: i32, data: &[u8]) {
    let mut event = mole_bss_types::wake_event::default();

    plain::copy_from_bytes(&mut event, data).expect("Data buffer was too short");

    let wakeup = ctx
        .entry((event.src_tgidpid, event.tgt_tgidpid))
        .or_insert(0);
    *wakeup += 1;
}

fn handle_lost_events(cpu: i32, count: u64) {
    eprintln!("Lost {} events on CPU {}", count, cpu);
}

pub fn record_wakeups(tgid: i32, duration: Duration, verbose: bool) -> Result<Wakeups> {
    let mut skel_builder = MoleSkelBuilder::default();
    if verbose {
        skel_builder.obj_builder.debug(true);
    }

    bump_memlock_rlimit()?;
    let mut open_skel = skel_builder.open()?;
    open_skel.rodata().tgid = tgid;

    let mut skel = open_skel.load()?;
    skel.attach()?;

    let mut ctx = Wakeups::new();
    {
        let perf = PerfBufferBuilder::new(skel.maps_mut().events())
            .sample_cb(|cpu: i32, data: &[u8]| {
                handle_event(&mut ctx, cpu, data);
            })
            .lost_cb(handle_lost_events)
            .build()?;

        let start = Instant::now();
        loop {
            perf.poll(Duration::from_millis(100))?;
            if Instant::now() - start > duration {
                break;
            }
        }
    }

    Ok(ctx)
}
