use std::collections::HashMap;
use std::env;
use std::fs;
use std::str::FromStr;
use std::thread;
use std::time::{Duration};

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
        "{}({}) usr {} sys {} vctxsw {} ivctxsw {} on_cpu {} wait_for_cpu {} slices {}",
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

    for curr in &n.threads {
        match p.threads.get(&curr.0) {
            Some(prev) => print_delta_threads(prev, curr.1),
            None => println!("new {}", curr.0),
        }
    }
}

fn main() {
    let mut args = env::args_os();
    println!("{:?}", args);

    let pid_arg = args.nth(1).unwrap().into_string().unwrap();
    let pid = i32::from_str(&pid_arg).expect("pid is not specified or can't be parsed");

    loop {
        let _prev_stat = procfs::read_stat();
        let prev = inspect_process(pid).expect("Can't find the process");
        thread::sleep(Duration::from_secs(1));
        let _curr_stat = procfs::read_stat();
        let curr = inspect_process(pid).expect("Can't find the process");

        print_delta_procs(&prev, &curr);
    }
}
