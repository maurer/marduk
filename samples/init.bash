#!/usr/bin/env bash
# NixOS clang sets this variable, and it interferes with g++
unset NIX_CXXSTDLIB_LINK

MAKEFLAGS=-j9

# Build artificial tests
make -C samples/artificial ${MAKEFLAGS}

JULIET=samples/Juliet-1.3/C/testcases/CWE416_Use_After_Free
JULIET_OUT_BASE=samples/Juliet-1.3/CWE416

# Build juliet true-positive baseline files
OMITGOOD=1 make -C ${JULIET} individuals ${MAKEFLAGS}
OMIT_OUT=${JULIET_OUT_BASE}/omit_good_individuals
mkdir -p ${OMIT_OUT}
cp ${JULIET}/*.out ${OMIT_OUT}

# Rebuild as false-positive bearing individuals
make -C ${JULIET} clean ${MAKEFLAGS}
make -C ${JULIET} individuals ${MAKEFLAGS}
IND_OUT=${JULIET_OUT_BASE}/individuals
mkdir -p ${IND_OUT}
cp ${JULIET}/*.out ${IND_OUT}

# And as one file, all together
make -C ${JULIET} ${MAKEFLAGS}
cp ${JULIET}/CWE416 ${JULIET_OUT_BASE}
