use std::cmp;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use structopt::StructOpt;

mod bpf;
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
        on_cpu: schedstat.on_cpu / 1000, // nanoseconds to microseconds
        waiting_for_cpu: schedstat.waiting_for_cpu / 1000, // nanoseconds to microseconds
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

    for tid in procfs::read_proc_threads(pid).expect("Can't find the specified process") {
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
    prev: &ProcessDataSnapshot,
    curr: &ProcessDataSnapshot,
    load: u64,
) {
    assert_eq!(prev.pid, curr.pid);

    let p_threads: HashSet<_> = prev.threads.keys().cloned().collect();
    let c_threads: HashSet<_> = curr.threads.keys().cloned().collect();
    let alive: HashSet<_> = p_threads.intersection(&c_threads).collect();
    let died: HashSet<_> = p_threads.difference(&c_threads).collect();
    let born: HashSet<_> = c_threads.difference(&p_threads).collect();

    println!(
        "{} threads, {} died, {} born",
        c_threads.len(),
        died.len(),
        born.len()
    );

    for pid in &alive {
        let p = prev.threads.get(&pid).unwrap();
        let c = curr.threads.get(&pid).unwrap();

        let on_cpu = c.on_cpu - p.on_cpu;
        let slices = c.slices - p.slices;
        let avg_slice = if slices > 0 { on_cpu / slices } else { 0 };

        table.add_row(vec![
            output::Data::Int(p.pid as i64),
            output::Data::Text(p.comm.clone()),
            output::Data::Float((c.utime - p.utime) as f64 / load as f64 * 100.0),
            output::Data::Float((c.stime - p.stime) as f64 / load as f64 * 100.0),
            output::Data::UInt(on_cpu),
            output::Data::UInt(c.waiting_for_cpu - p.waiting_for_cpu),
            output::Data::UInt(slices),
            output::Data::UInt(avg_slice),
            output::Data::UInt(c.vctxsw - p.vctxsw),
            output::Data::UInt(c.ivctxsw - p.ivctxsw),
        ]);
    }

    println!("{}", table.display_table());
    table.clear_data();
}

fn tgidpid_tgid(tgidpid: u64) -> i32 {
    (tgidpid >> 32) as i32
}

fn tgidpid_pid(tgidpid: u64) -> i32 {
    tgidpid as i32
}

fn print_top_events(map: &HashMap<i32, u64>, curr: &ProcessDataSnapshot) -> Vec<String> {
    let mut count_vec: Vec<_> = map.iter().collect();
    count_vec.sort_by(|a, b| b.1.cmp(a.1));
    let mut ret = vec![];
    let unknown = "unknown".to_string();

    let mut c = 0;
    for i in count_vec {
        if c > 20 {
            break;
        }
        c += 1;
        let pid = i.0;
        let comm = match curr.threads.get(pid) {
            Some(t) => &t.comm,
            None => &unknown,
        };
        ret.push(format!("{:<10} {} ({})", i.1, comm, pid));
        // 10 + 16 + 8 + 4 (two spaces and () ) = 38
    }

    ret
}

fn print_wakeups(wakeups: &bpf::Wakeups, curr: &ProcessDataSnapshot) {
    let mut inputs: HashMap<i32, u64> = HashMap::new();
    let mut outputs: HashMap<i32, u64> = HashMap::new();
    let mut wakees: HashMap<i32, u64> = HashMap::new();
    let mut wakers: HashMap<i32, u64> = HashMap::new();

    let tgid = curr.pid;

    for item in wakeups {
        let src = item.0 .0;
        let tgt = item.0 .1;
        let count = item.1;
        let tgid1 = tgidpid_tgid(src);
        let pid1 = tgidpid_pid(src);
        let tgid2 = tgidpid_tgid(tgt);
        let pid2 = tgidpid_pid(tgt);

        if tgid1 != tgid {
            assert_eq!(tgid2, tgid);

            let entry = inputs.entry(pid2).or_insert(0);
            *entry += count;
        }

        if tgid2 != tgid {
            assert_eq!(tgid1, tgid);

            let entry = outputs.entry(pid1).or_insert(0);
            *entry += count;
        }

        if tgid1 == tgid && tgid2 == tgid {
            let entry = wakers.entry(pid1).or_insert(0);
            *entry += count;
            let entry = wakees.entry(pid2).or_insert(0);
            *entry += count;
        }
    }

    let inputs = print_top_events(&inputs, &curr);
    let outputs = print_top_events(&outputs, &curr);
    let nr = cmp::max(inputs.len(), outputs.len());
    println!(
        "{:38} {:38}\n{}",
        "top inputs",
        "top outputs",
        &"-".repeat(38 + 38 + 1)
    );
    for i in 0..nr {
        println!(
            "{:38} {:38}",
            inputs.get(i).unwrap_or(&"".to_string()),
            outputs.get(i).unwrap_or(&"".to_string())
        );
    }
    println!("");

    let wakers = print_top_events(&wakers, &curr);
    let wakees = print_top_events(&wakees, &curr);
    let nr = cmp::max(wakers.len(), wakees.len());
    println!(
        "{:38} {:38}\n{}",
        "top wakees",
        "top wakers",
        &"-".repeat(38 + 38 + 1)
    );
    for i in 0..nr {
        println!(
            "{:38} {:38}",
            wakers.get(i).unwrap_or(&"".to_string()),
            wakees.get(i).unwrap_or(&"".to_string())
        );
    }
    println!("");
}

#[derive(Debug, StructOpt)]
struct CliArgs {
    #[structopt(short = "p", long, conflicts_with = "cmd")]
    pid: Option<i32>,

    // #[structopt(short="c", long, conflicts_with="pid")]
    // cmd: Option<String>,
    //
    #[structopt(short = "s", long)]
    sort_by: Option<String>,

    #[structopt(short = "f", long)]
    filter_by: Option<String>,

    #[structopt(short = "t", long, required = false, default_value = "1000")]
    sleep_ms: u64,

    #[structopt(short = "n", long)]
    top: Option<usize>,
}

fn main() {
    let mut table = table![
        ("pid", 8),
        ("comm", 16),
        ("usr%", 4),
        ("sys%", 4),
        ("on_cpu", 10),
        ("wait", 10),
        ("slices", 10),
        ("avg_slice", 10),
        ("vctxsw", 10),
        ("ivctxsw", 10)
    ];

    let args = CliArgs::from_args();
    table.top = args.top;

    if let Some(sort_by) = args.sort_by {
        table.sort_by = Some(
            table
                .column_index_by_desc(&sort_by)
                .expect("Invalid column specified"),
        );
    }

    if let Some(filter_by) = args.filter_by {
        table.filter_by = Some(
            table
                .column_index_by_desc(&filter_by)
                .expect("Invalid column specified"),
        );
    }

    let pid = args.pid.expect("Pid is not specififed");

    loop {
        let prev_stat = procfs::read_stat();
        let prev = inspect_process(pid).expect("Can't find the process");

        let wakeups =
            bpf::record_wakeups(pid, Duration::from_millis(args.sleep_ms), false).unwrap();

        let curr_stat = procfs::read_stat();
        let curr = inspect_process(pid).expect("Can't find the process");

        print_delta_procs(
            &mut table,
            &prev,
            &curr,
            system_load(&prev_stat, &curr_stat),
        );

        print_wakeups(&wakeups, &curr);
    }
}
