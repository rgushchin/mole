#ifndef __MOLE_H
#define __MOLE_H

struct wake_event {
	unsigned long src_tgidpid;
	unsigned long tgt_tgidpid;
};

#endif /* __MOLE_H */
