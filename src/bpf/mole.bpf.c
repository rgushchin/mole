#include "vmlinux.h"
#include "mole.h"

#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

const volatile pid_t tgid = 0;

// Dummy instance to get skeleton to generate definition for `struct event`
struct event _event = {0};

// Kernel 5.14 changed the state field to __state
struct task_struct___pre_5_14 {
	long int state;
};

struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 10240);
	__type(key, u32);
	__type(value, u64);
} start SEC(".maps");

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
	struct event event = {};
	pid_t tgt_tgid = BPF_CORE_READ(p, tgid);

	if (curr->tgid == tgid || tgt_tgid == tgid) {
		event.kind = 0;
		event.src_tgidpid = tgidpid(curr->tgid, curr->pid);
		event.tgt_tgidpid = tgidpid(tgt_tgid, BPF_CORE_READ(p, pid));

		bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU, &event,
				      sizeof(event));
	}

	return 0;
}

static __always_inline int trace_enqueue(u32 pid)
{
	u64 ts = bpf_ktime_get_ns();
	bpf_map_update_elem(&start, &pid, &ts, 0);
	return 0;
}

static inline long get_task_state(struct task_struct *t)
{
	if (bpf_core_field_exists(t->__state))
		return t->__state;

	return ((struct task_struct___pre_5_14*)t)->state;
}

SEC("tp_btf/sched_switch")
int mole_sched_switch(u64 *ctx)
{
	/* TP_PROTO(bool preempt, struct task_struct *prev,
	 *	    struct task_struct *next)
	 */
	struct task_struct *prev = (struct task_struct *)ctx[1];
	struct task_struct *next = (struct task_struct *)ctx[2];
	struct event event = {};
	u64 *tsp, delta_us;
	long state = get_task_state(prev);
	u32 pid;

	if (next->tgid == tgid)
		trace_enqueue(next->pid);

	if (prev->tgid == tgid) {
		pid = prev->pid;

		tsp = bpf_map_lookup_elem(&start, &pid);
		if (!tsp)
			return 0;

		delta_us = (bpf_ktime_get_ns() - *tsp) / 1000;

		event.kind = 1;
		event.src_tgidpid = pid;
		event.tgt_tgidpid = delta_us;

		bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU,
				      &event, sizeof(event));

		bpf_map_delete_elem(&start, &pid);
	}

	return 0;
}

char LICENSE[] SEC("license") = "GPL";
