#include <stdlib.h>
#include <stdio.h>

int main () {
  char* out = malloc(1);
  *out = 'a'; // good
  printf("good! %c\n", *out);
  free(out);
}
