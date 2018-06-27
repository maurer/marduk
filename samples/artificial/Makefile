CFLAGS=-O0 -fomit-frame-pointer
export hardeningDisable=all

TARGETS=reloop func link external.so simple safe path_sensitive remalloc loop ll link.o seq_call restale recurse undef_stack undef_edge field_overwrite

all: $(TARGETS)

external.so: external.c
	gcc -shared -fPIC $(CFLAGS) external.c -o external.so

link: external.so link.o

clean:
	rm -f $(TARGETS)
