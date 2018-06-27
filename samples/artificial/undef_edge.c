#include <stdlib.h>

char undef_dyn_fixed(char** s, int n) {
	s[n] = malloc(1);
	free(s[n]);
	return *(s[4096]);
}

char undef_fixed_dyn(char** s, int n) {
	s[4096] = malloc(1);
	char* r = s[n];
	free(s[4096]);
	return *r;
}

char undef_dyn_dyn(char **s, int n, int o) {
	s[n] = malloc(1);
	free(s[n]);
	return *(s[o]);
}

void main () {}
