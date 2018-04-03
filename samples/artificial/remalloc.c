#include <stdlib.h>

void main () {
  char* p = malloc(1);
  free(p);
  p = malloc(1);
  *p = 1;
}

char* f() {
	return malloc(1);
}

void ctx_sense() {
	char* p = f();
	free(p);
	p = f();
	*p = 1;
}
