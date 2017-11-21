CFLAGS=-fomit-frame-pointer
export hardeningDisable=all

TARGETS=func link external.so simple safe path_sensitive remalloc loop ll link.o

all: $(TARGETS)

external.so: external.c
	gcc -shared -fPIC external.c -o external.so

link: external.so link.o

clean:
	rm -f $(TARGETS)
