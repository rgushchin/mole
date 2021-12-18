use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

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

    let tasks = fs::read_dir(format!("/proc/{}/task/", pid)).ok()?;
    for task in tasks {
        let s = task.unwrap().file_name().into_string().unwrap();
        let tid = i32::from_str(&s).unwrap();
        if let Some(td) = inspect_thread(pid, tid) {
            ret.threads.insert(tid, td);
        }
    }

    Some(ret)
}

fn print_delta_threads(p: &ThreadDataSnapshot, n: &ThreadDataSnapshot) {
    assert_eq!(p.pid, n.pid);

    println!(
        "{:16} {:<8} {:<10} {:<10} {:<10} {:<10} {:<10} {:<10} {:<10}",
        p.comm,
        p.pid,
        n.utime - p.utime,
        n.stime - p.stime,
        n.vctxsw - p.vctxsw,
        n.ivctxsw - p.ivctxsw,
        n.on_cpu - p.on_cpu,
        n.waiting_for_cpu - p.waiting_for_cpu,
        n.slices - p.slices,
    );
}

fn print_delta_procs(p: &ProcessDataSnapshot, n: &ProcessDataSnapshot) {
    assert_eq!(p.pid, n.pid);

    let p_threads: HashSet<_> = p.threads.keys().cloned().collect();
    let n_threads: HashSet<_> = p.threads.keys().cloned().collect();
    let alive: HashSet<_> = p_threads.intersection(&n_threads).collect();
    let died: HashSet<_> = p_threads.difference(&n_threads).collect();
    let born: HashSet<_> = n_threads.intersection(&p_threads).collect();

    println!("{} threads, {} died, {} born", n_threads.len(), died.len(), born.len());

    println!(
        "{:16} {:8} {:10} {:10} {:10} {:10} {:10} {:10} {:10}",
        "comm", "pid", "usr%", "sys%", "vctxsw", "ivctxsw", "on_cpu", "wait", "slices"
    );
    println!("--------------------------------------------------------------------------------------------------");

    for pid in &alive {
        print_delta_threads(p.threads.get(&pid).unwrap(), n.threads.get(&pid).unwrap());
    }

    println!("");
}

fn main() {
    let mut args = env::args_os();
    println!("{:?}", args);

    let pid_arg = args.nth(1).unwrap().into_string().unwrap();
    let pid = i32::from_str(&pid_arg).expect("pid is not specified or can't be parsed");

    loop {
        let _prev_stat = procfs::read_stat();
        let prev = inspect_process(pid).expect("Can't find the process");
        thread::sleep(Duration::from_secs(5));
        let _curr_stat = procfs::read_stat();
        let curr = inspect_process(pid).expect("Can't find the process");

        print_delta_procs(&prev, &curr);
    }
}
