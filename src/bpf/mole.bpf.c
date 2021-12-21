#include "vmlinux.h"
#include "mole.h"

#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

const volatile pid_t tgid = 0;

// Dummy instance to get skeleton to generate definition for `struct event`
struct wake_event _event = {0};

struct {
	__uint(type, BPF_MAP_TYPE_PERF_EVENT_ARRAY);
	__uint(key_size, sizeof(u32));
	__uint(value_size, sizeof(u32));
} events SEC(".maps");

unsigned long tgidpid(pid_t tgid, pid_t pid)
{
	unsigned long ret = tgid;

	ret <<= 32;
	ret |= pid;

	return ret;
}

SEC("kprobe/try_to_wake_up")
int BPF_KPROBE(mole_handle_try_to_wake_up, struct task_struct *p,
	       unsigned int state, int wake_flags)
{
	struct task_struct *curr = bpf_get_current_task_btf();
	struct wake_event event = {};
	pid_t tgt_tgid = BPF_CORE_READ(p, tgid);

	if (curr->tgid == tgid || tgt_tgid == tgid) {
		event.src_tgidpid = tgidpid(curr->tgid, curr->pid);
		event.tgt_tgidpid = tgidpid(tgt_tgid, BPF_CORE_READ(p, pid));

		bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU, &event,
				      sizeof(event));
	}

	return 0;
}

/* SEC("tracepoint:sched:sched_switch") */
/* int BPF_KPROBE(mole_sched_switch, struct task_struct *p, unsigned int state, */
/* 	       int wake_flags) */
/* { */
/* } */

char LICENSE[] SEC("license") = "GPL";
