.global main
.intel_syntax noprefix
f:
	mov rdi, 1
	call malloc@PLT
	mov rdi, rax
	mov rbx, rax 
	call free@PLT
	mov rax, rbx
	ret
	.size f, .-f
	.type f, @function
g:
	mov rdi, 1
	call malloc@PLT
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
