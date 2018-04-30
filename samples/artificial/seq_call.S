.extern malloc
.global f, g, main
.intel_syntax noprefix
f:
	mov rdi, 1
	call malloc
	mov rdi, rax
	mov rbx, rax 
	call free
	mov rax, rbx
	ret
	.size f, .-f
	.type f, @function
g:
	mov rdi, 1
	call malloc
	ret
	.size g, .-g
	.type g, @function
main:
	call f
	call g
	movb [rax], 3
	ret
	.size main, .-main
	.type main, @function
