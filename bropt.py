"""Python interface for the bropt Brainfuck interpreter.

This module provides a simple wrapper around the `bropt_run` function
exported by the Rust library. The dynamic library is loaded from the
`target/release` directory produced by `cargo build --release`.
"""
from __future__ import annotations

import ctypes
from pathlib import Path

_lib_path = Path(__file__).resolve().parent / "target" / "release" / "libbropt.so"
_lib = ctypes.CDLL(str(_lib_path))

_lib.bropt_run.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.c_ubyte]
_lib.bropt_run.restype = None

def run(code: str, length: int = 65536, flush: bool = False) -> None:
    """Run a Brainfuck program.

    Parameters
    ----------
    code: str
        Brainfuck source code.
    length: int, optional
        Tape length (default 65536).
    flush: bool, optional
        Flush standard output after each output instruction.
    """
    _lib.bropt_run(code.encode("utf-8"), length, 1 if flush else 0)
