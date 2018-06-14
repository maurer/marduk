#include <stdlib.h>

void make_freed(char** foo) {
	char* bar = malloc(1);
	*foo = bar;
	free(bar);
}

int main() {
	char* foo;
	make_freed(&foo);
	*foo = 0;
}
