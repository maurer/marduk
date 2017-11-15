#include <stdlib.h>

char* foo(char* p) {
  if (*p) {
    free(p);
    return 0; //malloc(1);
  } else {
    return p;
  }
}

void main () {
  char* p = malloc(1);
  char* q = foo(p);
  *q = 1; 
}
