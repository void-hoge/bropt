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

## Experimental Result

- Benchmark programs and inputs are available in the `benches` directory.
- Average execution time of 100 runs, measured after a 100-run warm-up.
  - Benchmark script: [`benchmark.sh`](./benchmark.sh)
  - ThinkPad X13 Gen3 (Ryzen 7 PRO 6850U, 32GB RAM, WD Black SN770 1TB SSD)
  - Debian GNU/Linux

|            | `brust` [s] | `Tritium` [s] | `bffsree` [s] | `bropt` [s] |
|------------|-------------|---------------|---------------|-------------|
| awib       | **0.0197**  | 0.350         | SIGSEGV       | 0.0233      |
| Collatz    | 1.71        | 1.51          | 1.41          | **1.29**    |
| Counter    | 2.27        | **1.92**      | 2.49          | 1.96        |
| EasyOpt    | 0.0308      | **0.0200**    | 0.0353        | 0.0308      |
| Factor     | 1.66        | 2.24          | 1.98          | **1.58**    |
| Hanoi      | 0.112       | 0.0711        | **0.0159**    | 0.0177      |
| Life       | **0.0114**  | 0.0285        | 0.0117        | 0.0119      |
| Long       | 0.664       | 0.652         | 0.0654        | **0.0552**  |
| Mandelbrot | 1.52        | 1.95          | 1.44          | **1.38**    |
| Prime      | 1.12        | **0.102**     | 0.115         | 0.124       |
| SelfInt    | 2.05        | 2.59          | 1.89          | **1.64**    |
| Sudoku     | 0.724       | 0.741         | 1.03          | **0.692**   |
|------------|-------------|---------------|---------------|-------------|
| Fastest    | 2           | 3             | 1             | 6           |

- `Tritium` by rdebath
  - https://github.com/rdebath/Brainfuck
- `bffsree` by Sree Kotay 
  - Implementing brainfuck Part 1: http://sree.kotay.com/2013/02/implementing-brainfuck.html 
  - Implementing brainfuck Part 2: http://sree.kotay.com/2013/02/implementing-brainfuck-part-2.html
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
