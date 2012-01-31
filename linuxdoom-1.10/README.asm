
README - DOOM assembly code

Okay, I add the DOS assembly module for the historically
inclined here (may rec.games.programmer suffer). If anyone
feels the urge to port these to GNU GCC; either inline or
as separate modules including Makefile support, be my guest.

Module tmap.S includes the inner loops for texture mapping,
the interesting one being the floor/ceiling span rendering.

There was another module in the source dump, fpfunc.S, that
had both texture mapping and fixed point functions. It
contained implementations both for i386 and M68k. For
brevity, I include only the i386 fixed point stuff below.

//====================================================
// tmap.S  as of January 10th, 1997

//================
//
// R_DrawColumn
//
//================

	.data
loopcount	.long	0
pixelcount	.long	0

	.text

	.align 16
.globl _R_DrawColumn
_R_DrawColumn:

	pushad

	movl		ebp,[_dc_yl]
	movl		ebx,ebp
	movl     edi,[_ylookup+ebx*4]
	movl		ebx,[_dc_x]
	addl     edi,[_columnofs + ebx*4]

	movl		eax,[_dc_yh]
	incl		eax
	subl     eax,ebp                   	// pixel count
	movl		[pixelcount],eax			// save for final pixel
	js		done						// nothing to scale
	shrl		eax,1						// double pixel count
	movl		[loopcount],eax
	
	movl     ecx,[_dc_iscale]

	movl		eax,[_centery]
	subl		eax,ebp
	imull	ecx
	movl		ebp,[_dc_texturemid]
	subl		ebp,eax
	shll		ebp,9							// 7 significant bits, 25 frac

	movl     esi,[_dc_source]
	

	movl		ebx,[_dc_iscale]
	shll		ebx,9
	movl		eax,OFFSET patch1+2		// convice tasm to modify code...
	movl		[eax],ebx
	movl		eax,OFFSET patch2+2		// convice tasm to modify code...
	movl		[eax],ebx
	
// eax		aligned colormap
// ebx		aligned colormap
// ecx,edx	scratch
// esi		virtual source
// edi		moving destination pointer
// ebp		frac
	
	movl	ecx,ebp					// begin calculating first pixel
	addl	ebp,ebx					// advance frac pointer
	shrl ecx,25					// finish calculation for first pixel
	movl	edx,ebp					// begin calculating second pixel
	addl	ebp,ebx					// advance frac pointer
	shrl edx,25					// finish calculation for second pixel
	movl eax,[_dc_colormap]
	movl ebx,eax
	movb	al,[esi+ecx]			// get first pixel
	movb	bl,[esi+edx]			// get second pixel
	movb	al,[eax]				// color translate first pixel
	movb	bl,[ebx]				// color translate second pixel
	
	testl	[pixelcount],0fffffffeh
	jnz	doubleloop				// at least two pixels to map
	jmp	checklast
	
	.align	16
doubleloop:
	movl	ecx,ebp					// begin calculating third pixel
patch1:
	addl	ebp,12345678h			// advance frac pointer
	movb	[edi],al				// write first pixel
	shrl ecx,25					// finish calculation for third pixel
	movl	edx,ebp					// begin calculating fourth pixel
patch2:
	addl	ebp,12345678h			// advance frac pointer
	movl	[edi+SCREENWIDTH],bl	// write second pixel
	shrl edx,25					// finish calculation for fourth pixel
	movb	al,[esi+ecx]			// get third pixel
	addl	edi,SCREENWIDTH*2		// advance to third pixel destination
	movb	bl,[esi+edx]			// get fourth pixel
	decl	[loopcount]				// done with loop?
	movb	al,[eax]				// color translate third pixel
	movb	bl,[ebx]				// color translate fourth pixel
	jnz	doubleloop
	
// check for final pixel
checklast:
	testl	[pixelcount],1
	jz	done
	movb	[edi],al				// write final pixel
	
done:
	popad
	ret
	


//================
//
// R_DrawSpan
//
// Horizontal texture mapping
//
//================


	.align	16
.globl _R_DrawSpan
_R_DrawSpan:
	pushad

//
// find loop count
//	
	movl		eax,[_ds_x2]
	incl		eax
	subl     eax,[_ds_x1]               	// pixel count
	movl		[pixelcount],eax			// save for final pixel
	js		hdone						// nothing to scale
	shrl		eax,1						// double pixel count
	movl		[loopcount],eax

//
// build composite position
//
	movl	ebp,[_ds_xfrac]
	shll	ebp,10
	andl	ebp,0ffff0000h
	movl	eax,[_ds_yfrac]
	shrl	eax,6
	andl	eax,0ffffh
	orl	ebp,eax

	movl	esi,[_ds_source]

//
// calculate screen dest
//
	movl	edi,[_ds_y]
	movl	edi,[_ylookup+edi*4]
	movl	eax,[_ds_x1]
	addl edi,[_columnofs+eax*4]

//
// build composite step
//
	movl	ebx,[_ds_xstep]
	shll	ebx,10
	andl	ebx,0ffff0000h
	movl	eax,[_ds_ystep]
	shrl	eax,6
	andl	eax,0ffffh
	orl	ebx,eax

	movl		eax,OFFSET hpatch1+2		// convice tasm to modify code...
	movl		[eax],ebx
	movl		eax,OFFSET hpatch2+2		// convice tasm to modify code...
	movl		[eax],ebx
	
// eax		aligned colormap
// ebx		aligned colormap
// ecx,edx	scratch
// esi		virtual source
// edi		moving destination pointer
// ebp		frac
	
	shldl ecx,ebp,22				// begin calculating third pixel (y units)
	shldl ecx,ebp,6				// begin calculating third pixel (x units)
	addl	ebp,ebx					// advance frac pointer
	andl ecx,4095				// finish calculation for third pixel
	shldl edx,ebp,22				// begin calculating fourth pixel (y units)
	shldl edx,ebp,6				// begin calculating fourth pixel (x units)
	addl	ebp,ebx					// advance frac pointer
	andl edx,4095				// finish calculation for fourth pixel
	movl eax,[_ds_colormap]
	movl ebx,eax
	movb	al,[esi+ecx]			// get first pixel
	movb	bl,[esi+edx]			// get second pixel
	movb	al,[eax]				// color translate first pixel
	movb	bl,[ebx]				// color translate second pixel
	
	testl	[pixelcount],0fffffffeh
	jnz	hdoubleloop				// at least two pixels to map
	jmp	hchecklast
	

	.align	16
hdoubleloop:
	shldl ecx,ebp,22				// begin calculating third pixel (y units)
	shldl ecx,ebp,6				// begin calculating third pixel (x units)
hpatch1:
	addl	ebp,12345678h			// advance frac pointer
	movb	[edi],al				// write first pixel
	andl ecx,4095				// finish calculation for third pixel
	shldl edx,ebp,22				// begin calculating fourth pixel (y units)
	shldl edx,ebp,6				// begin calculating fourth pixel (x units)
hpatch2:
	addl	ebp,12345678h			// advance frac pointer
	movb	[edi+1],bl				// write second pixel
	andl edx,4095				// finish calculation for fourth pixel
	movb	al,[esi+ecx]			// get third pixel
	addl	edi,2					// advance to third pixel destination
	movb	bl,[esi+edx]			// get fourth pixel
	decl	[loopcount]				// done with loop?
	movb	al,[eax]				// color translate third pixel
	movb	bl,[ebx]				// color translate fourth pixel
	jnz	hdoubleloop

// check for final pixel
hchecklast:
	testl	[pixelcount],1
	jz	hdone
	movb	[edi],al				// write final pixel
	
hdone:
	popad
	ret




//====================================================
// fpfunc.S  as of January 10th, 1997 (parts)

#ifdef i386

.text
	.align 4
.globl _FixedMul
_FixedMul:	
	pushl %ebp
	movl %esp,%ebp
	movl 8(%ebp),%eax
	imull 12(%ebp)
	shrdl $16,%edx,%eax
	popl %ebp
	ret


	.align 4
.globl _FixedDiv2
_FixedDiv2:
	pushl %ebp
	movl %esp,%ebp
	movl 8(%ebp),%eax
	cdq
	shldl $16,%eax,%edx
	sall	$16,%eax
	idivl	12(%ebp)
	popl %ebp
	ret

#endif

