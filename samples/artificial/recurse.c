#include <stdlib.h>
char* g();
void h(char*);
void f();

char* g() {
	return malloc(1);
}
void h(char* x) {
	*x = 1;
	free(x);
	f();
}
void f() {
	h(g());
}
int main () {
	f();
}
