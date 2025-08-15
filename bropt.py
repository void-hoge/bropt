"""Python interface for the bropt Brainfuck interpreter.

This module provides a simple wrapper around the `bropt_run` function
exported by the Rust library. The dynamic library is loaded from the
`target/release` directory produced by `cargo build --release`.
"""
from __future__ import annotations

import ctypes
from pathlib import Path

try:  # optional numpy support
    import numpy as np
    _HAS_NUMPY = True
except Exception:  # pragma: no cover - numpy may be unavailable
    np = None
    _HAS_NUMPY = False

_lib_path = Path(__file__).resolve().parent / "target" / "release" / "libbropt.so"
_lib = ctypes.CDLL(str(_lib_path))

_lib.bropt_run.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.c_ubyte, ctypes.c_void_p]
_lib.bropt_run.restype = ctypes.c_char_p
_lib.bropt_free_error.argtypes = [ctypes.c_char_p]
_lib.bropt_free_error.restype = None

def run(code: str, length: int = 65536, flush: bool = False, *, as_numpy: bool = False):
    """Run a Brainfuck program and return the final tape state.

    Parameters
    ----------
    code: str
        Brainfuck source code.
    length: int, optional
        Tape length (default 65536).
    flush: bool, optional
        Flush standard output after each output instruction.
    as_numpy: bool, optional
        Return a :class:`numpy.ndarray` of dtype ``int8`` instead of ``bytes``.
    """
    buf = (ctypes.c_ubyte * length)()
    err = _lib.bropt_run(code.encode("utf-8"), length, 1 if flush else 0, ctypes.cast(buf, ctypes.c_void_p))
    if err:
        msg = ctypes.string_at(err).decode("utf-8")
        _lib.bropt_free_error(err)
        raise RuntimeError(msg)
    data = bytes(buf)
    if as_numpy:
        if not _HAS_NUMPY:
            raise RuntimeError("numpy is not available")
        return np.frombuffer(data, dtype=np.uint8).astype(np.int8)
    return data
