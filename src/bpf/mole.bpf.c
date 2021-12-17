#include "vmlinux.h"
#include "mole.h"

#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

const volatile __u64 min_us = 0;
const volatile pid_t targ_pid = 0;
const volatile pid_t targ_tgid = 0;

// Dummy instance to get skeleton to generate definition for `struct event`
struct event _event = {0};

struct {
	__uint(type, BPF_MAP_TYPE_PERF_EVENT_ARRAY);
	__uint(key_size, sizeof(u32));
	__uint(value_size, sizeof(u32));
} events SEC(".maps");

SEC("kprobe/try_to_wake_up")
int BPF_KPROBE(mole_handle_try_to_wake_up, struct task_struct *p, unsigned int state, int wake_flags)
{
	struct task_struct *curr = bpf_get_current_task_btf();
	struct event event = {};

	event.pid1 = curr->pid;
	event.pid2 = BPF_CORE_READ(p, pid);

	bpf_probe_read_kernel_str(&event.comm1, sizeof(event.comm1), curr->comm);
	bpf_probe_read_kernel_str(&event.comm2, sizeof(event.comm2), p->comm);

	bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU,
			      &event, sizeof(event));

	return 0;
}

SEC("tracepoint:sched:sched_switch")
int BPF_KPROBE(mole_sched_switch, struct task_struct *p, unsigned int state, int wake_flags)
{
}

char LICENSE[] SEC("license") = "GPL";
