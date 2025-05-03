# BROPT
- Yet another optimizing brainfuck interpreter in Rust
- bropt is probably the fastest non-JIT interpreter in most benchmarks as of May 2025.

## Build
```shellsession
$ git clone https://github.com/void-hoge/bropt.git
$ cd bropt
$ cargo build --release
```

## Usage
```shellsession
$ bropt -h
An optimizing brainfuck interpreter

Usage: bropt [OPTIONS] <FILE>

Arguments:
  <FILE>  Path to the Brainfuck program file to execute

Options:
  -l, --length <LENGTH>  Number of cells in the memory tape [default: 65536]
  -f, --flush            Flush stdout after each . instruction
  -h, --help             Print help
$
```

## Performance

- Benchmark programs and inputs are available in the `benches` directory.
- Average execution time of 100 runs, measured after a 100-run warm-up.
  - ThinkPad X13 Gen3 (Ryzen 7 PRO 6850U, 32GB RAM, WD Black SN770 1TB SSD)
  - Debian GNU/Linux

|            | `bffsree` [s] | `BrainForked` [s] | `brust` [s] | `bropt` [s] |
|------------|---------------|-------------------|-------------|-------------|
| awib       | SIGSEGV       | SIGSEGV           | **0.0197**  | 0.0220      |
| Collatz    | **1.41**      | 1.75              | 1.71        | 1.51        |
| Counter    | 2.49          | 2.58              | 2.27        | **1.90**    |
| EasyOpt    | 0.0353        | 0.0573            | 0.0308      | **0.0304**  |
| Factor     | 1.98          | 2.95              | 1.66        | **1.60**    |
| Hanoi      | **0.0159**    | 0.0167            | 0.112       | 0.0163      |
| Life       | 0.0117        | 0.0231            | **0.0114**  | **0.0114**  |
| Long       | 0.0654        | 0.731             | 0.664       | **0.0450**  |
| Mandelbrot | 1.44          | 1.41              | 1.52        | **1.40**    |
| Prime      | 0.115         | 2.71              | 1.12        | **0.104**   |
| SelfInt    | 1.89          | 2.19              | 2.05        | **1.63**    |
| Sudoku     | 1.03          | SIGSEGV           | 0.724       | **0.603**   |
|------------|---------------|-------------------|-------------|-------------|
| Fastest    | 2             | 0                 | 2           | 9           |

- `bffsree` by Sree Kotay 
  - Implementing brainfuck Part 1: http://sree.kotay.com/2013/02/implementing-brainfuck.html 
  - Implementing brainfuck Part 2: http://sree.kotay.com/2013/02/implementing-brainfuck-part-2.html
- `BrainForked` by John Griffin
  - https://github.com/JohnCGriffin/BrainForked
- `brust` by Mugi Noda (my previous work)
  - https://github.com/void-hoge/bropt
  - https://qiita.com/voidhoge/items/a4ca5888a624523906ae
- `bropt` (this work)

## Optimization
- Run-length compression of `+`/`-` and `<`/`>` instructions
- Folding of the reset idiom `[-]`
- Folding of the multi-target constant-multiplication idiom `[->+>++<<]`
- Folding of the zero-seeking idiom `[<<]`
- Folding of the zero-seeking idiom with side-effects `[-<<]`
- Removal of redundant write instructions
- Hoisting and transformation of reset idioms.

In addition to these foldings, pointer movements and increments are embedded into adjacent instructions to increase code density in memory.
For example, `>>>>>[-]++++>>>>` is compiled into a single (8 bytes) instruction.

## Author
- Mugi Noda (void-hoge)

## License
- GPLv3
  - This does not apply to the programs used in the benchmarks.
