#include <stdlib.h>
#include <stdio.h>

struct ll {
	struct ll *next;
	char* pay;
};
// Check that we don't get flow-jammed
void flow_loop() {	
	char* p = malloc(1);
	struct ll* cur = NULL;
	struct ll* cons;
	int i = 0;
	while (i < 3) {
		cons = malloc(sizeof(struct ll));
		cons->next = cur;
		cons->pay = malloc(2);
		cons->pay[0] = '.';
		cons->pay[1] = 0;
		cur = cons;
	};
	while (cur != NULL) {
		puts(cur->pay);
		free(cur->pay);
		cons = cur->next;
		free(cur);
		cur = cons;
	}
}

int main () {
  flow_loop();
}
