#include <stdlib.h>
int main() {
	while(1) {
		char* dummy = malloc(1);
		*dummy = 3;
		free(dummy);
	}
}
