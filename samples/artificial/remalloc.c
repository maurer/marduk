#include <stdlib.h>

void main () {
  char* p = malloc(1);
  free(p);
  p = malloc(1);
  *p = 1;
}
