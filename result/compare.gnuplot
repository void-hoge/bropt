set terminal pngcairo size 1280,720 enhanced font "Consolas,20"
set output "bfi_comp.png"

set title "Performance Comparison of Brainfuck Interpreters"
set style data histograms
set style histogram clustered gap 2
set style fill solid 1 border -1
set boxwidth 1

set ylabel "Execution Time (s)"
set grid ytics lt 1 lw 1 lc rgb "#808080"
set mxtics 1

set xtics rotate by -30

set key top right
set key font "Consolas,12"

plot 'data.txt' \
     using 2:xtic(1) title 'brust', \
     '' using 3 title 'Tritium', \
     '' using 4 title 'bffsree', \
     '' using 5 title 'bropt'
