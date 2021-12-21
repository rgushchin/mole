use std::fs;
use std::io;
use std::str::FromStr;

#[derive(Debug)]
pub struct StatData {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
    pub guest: u64,
    pub guest_nice: u64,
}

pub fn read_stat() -> StatData {
    let raw = fs::read_to_string(format!("/proc/stat")).unwrap();
    for line in raw.lines() {
        let mut items = line.split_whitespace();

        // cpu  38949072 159668 8359409 421823496 244797 2013797 1074640 0 278514 0
        items.next();

        let data = StatData {
            user: u64::from_str(items.next().unwrap()).unwrap(),
            nice: u64::from_str(items.next().unwrap()).unwrap(),
            system: u64::from_str(items.next().unwrap()).unwrap(),
            idle: u64::from_str(items.next().unwrap()).unwrap(),
            iowait: u64::from_str(items.next().unwrap()).unwrap(),
            irq: u64::from_str(items.next().unwrap()).unwrap(),
            softirq: u64::from_str(items.next().unwrap()).unwrap(),
            steal: u64::from_str(items.next().unwrap()).unwrap(),
            guest: u64::from_str(items.next().unwrap()).unwrap(),
            guest_nice: u64::from_str(items.next().unwrap()).unwrap(),
        };

        return data;
    }

    panic!("Can't read /proc/stat");
}

#[derive(Debug)]
pub struct ProcStatData {
    pub utime: u64,
    pub stime: u64,
}

fn parse_proc_stat(f: &str) -> Option<ProcStatData> {
    let raw = fs::read_to_string(f).ok()?;
    let mut items = raw.split_whitespace();

    // (0) pid  %d
    // (1) comm  %s
    // (2) state  %c
    // (3) ppid  %d
    // (4) pgrp  %d
    // (5) session  %d
    // (6) tty_nr  %d
    // (7) tpgid  %d
    // (8) flags  %u
    // (9) minflt  %lu
    // (10) cminflt  %lu
    // (11) majflt  %lu
    // (12) cmajflt  %lu
    // (13) utime  %lu
    // (14) stime  %lu
    // (15) cutime  %ld
    // (16) cstime  %ld
    // (17) priority  %ld
    // (18) nice  %ld
    // (19) num_threads  %ld
    // (20) itrealvalue  %ld
    // (21) starttime  %llu
    // (22) vsize  %lu
    // (23) rss  %ld
    // (24) rsslim  %lu
    // (25) startcode  %lu  [PT]
    // (26) endcode  %lu  [PT]
    // (27) startstack  %lu  [PT]
    // (28) kstkesp  %lu  [PT]
    // (29) kstkeip  %lu  [PT]
    // (30) signal  %lu
    // (31) blocked  %lu
    // (32) sigignore  %lu
    // (33) sigcatch  %lu
    // (34) wchan  %lu  [PT]
    // (35) nswap  %lu
    // (36) cnswap  %lu
    // (37) exit_signal  %d  (since Linux 2.1.22)
    // (38) processor  %d  (since Linux 2.2.8)
    // (39) rt_priority  %u  (since Linux 2.5.19)
    // (40) policy  %u  (since Linux 2.5.19)
    // (41) delayacct_blkio_ticks  %llu  (since Linux 2.6.18)
    // (42) guest_time  %lu  (since Linux 2.6.24)
    // (43) cguest_time  %ld  (since Linux 2.6.24)
    // (44) start_data  %lu  (since Linux 3.3)  [PT]
    // (45) end_data  %lu  (since Linux 3.3)  [PT]
    // (46) start_brk  %lu  (since Linux 3.3)  [PT]
    // (47) arg_start  %lu  (since Linux 3.5)  [PT]
    // (48) arg_end  %lu  (since Linux 3.5)  [PT]
    // (49) env_start  %lu  (since Linux 3.5)  [PT]
    // (50) env_end  %lu  (since Linux 3.5)  [PT]
    // (51) exit_code  %d  (since Linux 3.5)  [PT]

    let data = ProcStatData {
        utime: u64::from_str(items.nth(13).unwrap()).unwrap(),
        stime: u64::from_str(items.nth(0).unwrap()).unwrap(),
    };

    Some(data)
}

//
// /proc/pid/stat is different to /proc/tgid/task/pid/stat
//
pub fn read_proc_stat(pid: i32) -> Option<ProcStatData> {
    parse_proc_stat(&format!("/proc/{}/stat", pid))
}

pub fn read_thread_stat(tgid: i32, pid: i32) -> Option<ProcStatData> {
    parse_proc_stat(&format!("/proc/{}/task/{}/stat", tgid, pid))
}

#[derive(Debug)]
pub struct ProcStatusData {
    pub name: String,
    pub vctxsw: u64,
    pub ivctxsw: u64,
}

pub fn read_proc_status(pid: i32) -> Option<ProcStatusData> {
    let mut data = ProcStatusData {
        name: String::new(),
        vctxsw: 0,
        ivctxsw: 0,
    };

    let raw = fs::read_to_string(format!("/proc/{}/status", pid)).ok()?;
    for line in raw.lines() {
        if line.starts_with("Name:") {
            data.name = line.split_whitespace().nth(1).unwrap().to_string();
        } else if line.starts_with("voluntary_ctxt_switches:") {
            data.vctxsw = u64::from_str(line.split_whitespace().nth(1).unwrap()).unwrap();
        } else if line.starts_with("nonvoluntary_ctxt_switches:") {
            data.ivctxsw = u64::from_str(line.split_whitespace().nth(1).unwrap()).unwrap();
        }
    }

    Some(data)
}

#[derive(Debug)]
pub struct ProcSchedstatData {
    pub on_cpu: u64,          // se.sum_exec_runtime
    pub waiting_for_cpu: u64, // sched_info.run_delay
    pub slices: u64,          // sched_info.pcount
}

pub fn read_proc_schedstat(pid: i32) -> Option<ProcSchedstatData> {
    let raw = fs::read_to_string(format!("/proc/{}/schedstat", pid)).ok()?;
    let mut items = raw.split_whitespace();

    let data = ProcSchedstatData {
        on_cpu: u64::from_str(items.next().unwrap()).unwrap(),
        waiting_for_cpu: u64::from_str(items.next().unwrap()).unwrap(),
        slices: u64::from_str(items.next().unwrap()).unwrap(),
    };

    Some(data)
}

pub struct ProcTask {
    dir: io::Result<std::fs::ReadDir>,
}

impl Iterator for ProcTask {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        let mut next = None;
        if let Ok(dir) = &mut self.dir {
            if let Some(s) = dir.next() {
                if let Ok(s) = s {
                    let s = s.file_name().into_string().unwrap();
                    let tid = i32::from_str(&s).unwrap();
                    next = Some(tid);
                }
            }
        }
        next
    }
}

pub fn read_proc_threads(pid: i32) -> ProcTask {
    ProcTask {
        dir: fs::read_dir(format!("/proc/{}/task/", pid)),
    }
}
