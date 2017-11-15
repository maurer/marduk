#include <stdlib.h>
char* f() {
  return malloc(1);
}
void g(char* x) {
  free(x);
}
int main () {
  char* ptr = f();
  *ptr = 'a';
  g(ptr);
  *ptr = 'b';
}
