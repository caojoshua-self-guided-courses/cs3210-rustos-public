all:
	aarch64-elf-gcc -c -o sd.o sd.c
	aarch64-elf-gcc -c -o delays.o delays.c
	aarch64-elf-ar rcs libsd.a sd.o delays.o
	cp libsd.a ../.cargo/libsd.a

clean:
	rm *.o *.a
