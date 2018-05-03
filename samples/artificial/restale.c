#include <stdlib.h>

char* f() {
	return malloc(1);
}

int main () {
	char* x;
	while (1) {
		x = f();
		*x = 1;
		free(x);
	}		
}
