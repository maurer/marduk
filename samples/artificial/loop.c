#include <stdlib.h>
#include <stdio.h>

// Check that the program doesn't stall out if an infinite trace realizes the bug
void loop_bad() {
	char* p = malloc(1);
	*p = 1;
	while (*p < 127) {
		*p *= 3;
	}
	free(p);
	*p = 2;
}

void inc(char* p) {
	*p++;
}

// Check that the program will still walk through repeated addresses if that's the only way
void double_bad() {
	char* p = malloc(1);
	inc(p);
	inc(p);
	free(p);
	*p = 2;
}

int main () {
  loop_bad();
  double_bad();
}
