#include <stdlib.h>
void main () {}

char field_overwrite(char** s, int n) {
	free(s[1]);
	// This should be an error - condition is just to make sure
	// the compiler doesn't do optimization, but n=1 is legal
	if (n % 30 == 1) {
		return *(s[n]);
	}
	s[1] = malloc(1);
	// This should be safe
	return *(s[n]);
}
