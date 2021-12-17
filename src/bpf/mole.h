#ifndef __MOLE_H
#define __MOLE_H

#define TASK_COMM_LEN 16

struct event {
	pid_t pid1;
	pid_t pid2;
	u8 comm1[TASK_COMM_LEN];
	u8 comm2[TASK_COMM_LEN];
};

#endif /* __MOLE_H */
