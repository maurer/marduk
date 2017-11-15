#include <stdlib.h>
char* h() {
  return malloc(1);
}
void i(char* q) {
  free(q);
}
void j(char* q) {
  *q = 'a';
}
