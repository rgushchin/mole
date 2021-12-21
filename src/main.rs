use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;
extern crate structopt;
use structopt::StructOpt;

mod output;
mod procfs;

#[derive(Debug)]
struct ThreadDataSnapshot {
    pid: i32,
    comm: String,
    utime: u64,
    stime: u64,
    vctxsw: u64,
    ivctxsw: u64,
    on_cpu: u64,
    waiting_for_cpu: u64,
    slices: u64,
}

fn inspect_thread(tgid: i32, pid: i32) -> Option<ThreadDataSnapshot> {
    let stat = procfs::read_thread_stat(tgid, pid)?;
    let status = procfs::read_proc_status(pid)?;
    let schedstat = procfs::read_proc_schedstat(pid)?;

    let ret = ThreadDataSnapshot {
        pid: pid,
        comm: status.name,
        utime: stat.utime,
        stime: stat.stime,
        vctxsw: status.vctxsw,
        ivctxsw: status.ivctxsw,
        on_cpu: schedstat.on_cpu,
        waiting_for_cpu: schedstat.waiting_for_cpu,
        slices: schedstat.slices,
    };

    Some(ret)
}

struct ProcessDataSnapshot {
    pid: i32,
    threads: HashMap<i32, ThreadDataSnapshot>,
}

fn inspect_process(pid: i32) -> Option<ProcessDataSnapshot> {
    let mut ret = ProcessDataSnapshot {
        pid: pid,
        threads: HashMap::new(),
    };

    for tid in procfs::read_proc_threads(pid) {
        if let Some(td) = inspect_thread(pid, tid) {
            ret.threads.insert(tid, td);
        }
    }

    Some(ret)
}

fn system_load(p: &procfs::StatData, n: &procfs::StatData) -> u64 {
    let total = n.user
        + n.nice
        + n.system
        + n.idle
        + n.iowait
        + n.irq
        + n.softirq
        + n.steal
        + n.guest
        + n.guest_nice
        - p.user
        - p.nice
        - p.system
        - p.idle
        - p.iowait
        - p.irq
        - p.softirq
        - p.steal
        - p.guest
        - p.guest_nice;
    let idle = n.idle - p.idle;

    total - idle
}

fn print_delta_procs(
    table: &mut output::Table,
    p: &ProcessDataSnapshot,
    n: &ProcessDataSnapshot,
    load: u64,
) {
    assert_eq!(p.pid, n.pid);

    let p_threads: HashSet<_> = p.threads.keys().cloned().collect();
    let n_threads: HashSet<_> = p.threads.keys().cloned().collect();
    let alive: HashSet<_> = p_threads.intersection(&n_threads).collect();
    let died: HashSet<_> = p_threads.difference(&n_threads).collect();
    let born: HashSet<_> = n_threads.difference(&p_threads).collect();

    println!(
        "{} threads, {} died, {} born",
        n_threads.len(),
        died.len(),
        born.len()
    );

    for pid in &alive {
        let p = p.threads.get(&pid).unwrap();
        let n = n.threads.get(&pid).unwrap();

        table.add_row(vec![
            output::Data::Int(p.pid as i64),
            output::Data::Text(p.comm.clone()),
            output::Data::Float((n.utime - p.utime) as f64 / load as f64 * 100.0),
            output::Data::Float((n.stime - p.stime) as f64 / load as f64 * 100.0),
            output::Data::UInt(n.vctxsw - p.vctxsw),
            output::Data::UInt(n.ivctxsw - p.ivctxsw),
            output::Data::UInt(n.on_cpu - p.on_cpu),
            output::Data::UInt(n.waiting_for_cpu - p.waiting_for_cpu),
            output::Data::UInt(n.slices - p.slices),
        ]);
    }

    println!("{}", table.display_table());
    table.clear_data();
}

#[derive(Debug, StructOpt)]
struct CliArgs {
    #[structopt(short = "p", long, conflicts_with = "cmd")]
    pid: Option<i32>,

    // #[structopt(short="c", long, conflicts_with="pid")]
    // cmd: Option<String>,
    #[structopt(short = "s", long)]
    sort_by: Option<String>,

    #[structopt(short = "t", long, required = false, default_value = "1000")]
    sleep_ms: u64,
}

fn main() {
    let mut table = table![
        ("pid", 8),
        ("comm", 16),
        ("usr%", 4),
        ("sys%", 4),
        ("vctxsw", 10),
        ("ivctxsw", 10),
        ("on_cpu", 10),
        ("wait", 10),
        ("slices", 10)
    ];

    let args = CliArgs::from_args();

    if let Some(sort_by) = args.sort_by {
        table.sort_by = Some(
            table
                .column_index_by_desc(&sort_by)
                .expect("Invalid column specified"),
        );
    }

    let pid = args.pid.unwrap();

    loop {
        let prev_stat = procfs::read_stat();
        let prev = inspect_process(pid).expect("Can't find the process");
        thread::sleep(Duration::from_millis(args.sleep_ms));
        let curr_stat = procfs::read_stat();
        let curr = inspect_process(pid).expect("Can't find the process");

        print_delta_procs(
            &mut table,
            &prev,
            &curr,
            system_load(&prev_stat, &curr_stat),
        );
    }
}
