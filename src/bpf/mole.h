#ifndef __MOLE_H
#define __MOLE_H

struct event {
	unsigned long kind;
	unsigned long src_tgidpid;
	unsigned long tgt_tgidpid;
};

#endif /* __MOLE_H */
